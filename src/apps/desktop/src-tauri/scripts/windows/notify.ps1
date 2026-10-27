param([string]$TitleBase64, [string]$BodyBase64, [string]$Sound)
$ErrorActionPreference = 'Stop'
$Title = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($TitleBase64))
$Body = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($BodyBase64))
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null
$doc = New-Object Windows.Data.Xml.Dom.XmlDocument
$escapedTitle = [System.Security.SecurityElement]::Escape($Title)
$escapedBody = [System.Security.SecurityElement]::Escape($Body)
$audio = if ($Sound -eq 'true') { '<audio src="ms-winsoundevent:Notification.Default"/>' } else { '<audio silent="true"/>' }
$doc.LoadXml('<toast><visual><binding template="ToastGeneric"><text>' + $escapedTitle + '</text><text>' + $escapedBody + '</text></binding></visual>' + $audio + '</toast>')
$toast = [Windows.UI.Notifications.ToastNotification]::new($doc)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('com.agenthud.next.preview').Show($toast)
