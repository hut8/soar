import SwiftUI

struct ContentView: View {
    @StateObject private var trackingService = TrackingService()
    @StateObject private var authRepo = ObservableAuthState()

    var body: some View {
        Group {
            if authRepo.isLoggedIn {
                MainTabView()
                    .environmentObject(trackingService)
                    .environmentObject(authRepo)
            } else {
                LoginScreen()
                    .environmentObject(authRepo)
            }
        }
    }
}

// MARK: - Main Tab View

struct MainTabView: View {
    @EnvironmentObject var trackingService: TrackingService

    var body: some View {
        TabView {
            TrackerScreen()
                .tabItem {
                    Label("Tracker", systemImage: "location.fill")
                }

            LiveViewScreen()
                .tabItem {
                    Label("Live View", systemImage: "airplane")
                }
        }
    }
}

// MARK: - Observable Auth State

/// Wraps TokenManager in an ObservableObject so SwiftUI reacts to login/logout.
class ObservableAuthState: ObservableObject {
    @Published var isLoggedIn: Bool
    @Published var userName: String?

    private let authRepo = AuthRepository()

    init() {
        let tm = TokenManager.shared
        self.isLoggedIn = tm.isLoggedIn
        self.userName = tm.userName
    }

    func login(serverURL: String, email: String, password: String) async throws {
        let name = try await authRepo.login(serverURL: serverURL, email: email, password: password)
        await MainActor.run {
            self.userName = name
            self.isLoggedIn = true
        }
    }

    func requestPasswordReset(serverURL: String, email: String) async throws {
        try await authRepo.requestPasswordReset(serverURL: serverURL, email: email)
    }

    func logout() {
        authRepo.logout()
        isLoggedIn = false
        userName = nil
    }

    func createAuthenticatedAPI() -> SoarAPI? {
        return authRepo.createAuthenticatedAPI()
    }
}
