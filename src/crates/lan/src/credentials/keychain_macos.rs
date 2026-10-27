//! LAN grants' secrets in the login Keychain, never synchronized.
use super::{Result, SecretStore};

pub struct KeychainSecrets;
impl SecretStore for KeychainSecrets {
    fn read(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>> {
        let found = {
            let mut options = security_framework::passwords::PasswordOptions::new_generic_password(
                service, account,
            );
            options.set_access_synchronized(Some(false));
            security_framework::passwords::generic_password(options)
        };
        match found {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.code() == -25300 => Ok(None),
            Err(_) => Err("Keychain LAN credential read failed".into()),
        }
    }
    fn write(&self, service: &str, account: &str, bytes: &[u8]) -> Result<()> {
        use core_foundation::{
            base::{CFType, TCFType},
            boolean::CFBoolean,
            data::CFData,
            dictionary::CFDictionary,
            string::{CFString, CFStringRef},
        };
        use security_framework_sys::{
            item::*,
            keychain_item::{SecItemAdd, SecItemUpdate},
        };
        extern "C" {
            static kSecAttrAccessible: CFStringRef;
        }
        fn key(value: CFStringRef) -> CFType {
            unsafe { CFString::wrap_under_get_rule(value) }.as_CFType()
        }
        unsafe {
            let pairs = [
                (key(kSecClass), key(kSecClassGenericPassword)),
                (key(kSecAttrService), CFString::new(service).as_CFType()),
                (key(kSecAttrAccount), CFString::new(account).as_CFType()),
                (
                    key(kSecAttrSynchronizable),
                    CFBoolean::false_value().as_CFType(),
                ),
            ];
            let query = CFDictionary::from_CFType_pairs(&pairs);
            let data = CFData::from_buffer(bytes);
            let values = CFDictionary::from_CFType_pairs(&[(key(kSecValueData), data.as_CFType())]);
            let status = SecItemUpdate(query.as_concrete_TypeRef(), values.as_concrete_TypeRef());
            if status == 0 {
                return Ok(());
            }
            if status != -25300 {
                return Err("Keychain LAN credential write failed".into());
            }
            let mut insertion = pairs.to_vec();
            insertion.push((key(kSecValueData), data.as_CFType()));
            insertion.push((key(kSecAttrAccessible), key(
                security_framework_sys::access_control::kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly)));
            let insertion = CFDictionary::from_CFType_pairs(&insertion);
            if SecItemAdd(insertion.as_concrete_TypeRef(), std::ptr::null_mut()) == 0 {
                Ok(())
            } else {
                Err("Keychain LAN credential write failed".into())
            }
        }
    }
}
