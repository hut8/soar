import Foundation
import Security

/// Manages secure storage of authentication tokens and user preferences using the iOS Keychain.
class TokenManager {
    static let shared = TokenManager()

    private let serviceName = "com.soar.tracker"
    private let tokenKey = "jwt_token"
    private let serverURLKey = "server_url"
    private let userNameKey = "user_name"

    private let defaults = UserDefaults.standard

    private init() {}

    // MARK: - Token (Keychain)

    var token: String? {
        get { readKeychain(key: tokenKey) }
        set {
            if let value = newValue {
                saveKeychain(key: tokenKey, value: value)
            } else {
                deleteKeychain(key: tokenKey)
            }
        }
    }

    // MARK: - Server URL (UserDefaults — not sensitive)

    var serverURL: String {
        get { defaults.string(forKey: serverURLKey) ?? Server.production.url }
        set { defaults.set(newValue, forKey: serverURLKey) }
    }

    // MARK: - User Name (UserDefaults — not sensitive)

    var userName: String? {
        get { defaults.string(forKey: userNameKey) }
        set { defaults.set(newValue, forKey: userNameKey) }
    }

    // MARK: - Convenience

    var isLoggedIn: Bool {
        token != nil
    }

    func clear() {
        token = nil
        userName = nil
    }

    // MARK: - Keychain Helpers

    private func saveKeychain(key: String, value: String) {
        let data = Data(value.utf8)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key,
        ]
        // Delete existing item first
        SecItemDelete(query as CFDictionary)

        var addQuery = query
        addQuery[kSecValueData as String] = data
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        SecItemAdd(addQuery as CFDictionary, nil)
    }

    private func readKeychain(key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    private func deleteKeychain(key: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key,
        ]
        SecItemDelete(query as CFDictionary)
    }
}

// MARK: - Server Configuration

enum Server: String, CaseIterable {
    case production
    case staging

    var url: String {
        switch self {
        case .production: return "https://glider.flights"
        case .staging: return "https://staging.glider.flights"
        }
    }

    var label: String {
        switch self {
        case .production: return "Production"
        case .staging: return "Staging"
        }
    }

    static func from(url: String) -> Server {
        if url.contains("staging") { return .staging }
        return .production
    }
}
