// swift-tools-version: 6.0
// Computes the expected outputs of the shared parity fixtures with agent-hud-native-open's own code.
// AGENT_HUD_NATIVE_OPEN names that checkout; by default it sits beside this repository.
import Foundation
import PackageDescription

let open = ProcessInfo.processInfo.environment["AGENT_HUD_NATIVE_OPEN"] ?? "../../../../agent-hud-native-open"
let identity = URL(fileURLWithPath: open).standardizedFileURL.lastPathComponent.lowercased()

let package = Package(
    name: "parity-oracle",
    platforms: [.macOS(.v14)],
    dependencies: [.package(path: open)],
    targets: [
        .executableTarget(
            name: "parity-oracle",
            dependencies: [.product(name: "AgentHUDCore", package: identity)],
            swiftSettings: [.swiftLanguageMode(.v6)]
        ),
    ]
)
