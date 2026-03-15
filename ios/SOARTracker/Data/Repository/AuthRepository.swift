import Foundation

class AuthRepository {
    private let tokenManager = TokenManager.shared

    func login(serverURL: String, email: String, password: String) async throws -> String {
        let api = SoarAPI(baseURL: serverURL)
        let response = try await api.login(email: email, password: password)

        // Store credentials
        tokenManager.token = response.token
        tokenManager.serverURL = serverURL

        // Build display name
        let user = response.user
        let name: String
        if let first = user.firstName, let last = user.lastName, !first.isEmpty {
            name = "\(first) \(last)".trimmingCharacters(in: .whitespaces)
        } else {
            name = user.email
        }
        tokenManager.userName = name

        return name
    }

    func requestPasswordReset(serverURL: String, email: String) async throws {
        let api = SoarAPI(baseURL: serverURL)
        try await api.requestPasswordReset(email: email)
    }

    func logout() {
        tokenManager.clear()
    }

    func createAuthenticatedAPI() -> SoarAPI? {
        guard let token = tokenManager.token else { return nil }
        return SoarAPI(baseURL: tokenManager.serverURL, token: token)
    }
}
