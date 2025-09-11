import type { User } from '$lib/stores/auth';

const API_BASE = '/data';

export interface LoginRequest {
	email: string;
	password: string;
}

export interface LoginResponse {
	token: string;
	user: User;
}

export interface RegisterRequest {
	first_name: string;
	last_name: string;
	email: string;
	password: string;
	club_id?: string;
}

export interface PasswordResetRequest {
	email: string;
}

export interface PasswordResetConfirm {
	token: string;
	new_password: string;
}

export interface EmailVerificationConfirm {
	token: string;
}

class AuthApiError extends Error {
	constructor(
		message: string,
		public status: number
	) {
		super(message);
		this.name = 'AuthApiError';
	}
}

async function apiCall<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
	const response = await fetch(`${API_BASE}${endpoint}`, {
		...options,
		headers: {
			'Content-Type': 'application/json',
			...options.headers
		}
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new AuthApiError(errorText || 'Request failed', response.status);
	}

	if (response.status === 204) {
		return {} as T;
	}

	return response.json();
}

export const authApi = {
	async login(credentials: LoginRequest): Promise<LoginResponse> {
		return apiCall<LoginResponse>('/auth/login', {
			method: 'POST',
			body: JSON.stringify(credentials)
		});
	},

	async register(userData: RegisterRequest): Promise<void> {
		await apiCall<void>('/auth/register', {
			method: 'POST',
			body: JSON.stringify(userData)
		});
	},

	async getCurrentUser(token: string): Promise<User> {
		return apiCall<User>('/auth/me', {
			headers: {
				Authorization: `Bearer ${token}`
			}
		});
	},

	async requestPasswordReset(data: PasswordResetRequest): Promise<void> {
		await apiCall<void>('/auth/password-reset/request', {
			method: 'POST',
			body: JSON.stringify(data)
		});
	},

	async confirmPasswordReset(data: PasswordResetConfirm): Promise<void> {
		await apiCall<void>('/auth/password-reset/confirm', {
			method: 'POST',
			body: JSON.stringify(data)
		});
	},

	async verifyEmail(token: string): Promise<void> {
		await apiCall<void>('/auth/verify-email', {
			method: 'POST',
			body: JSON.stringify({ token })
		});
	}
};

export { AuthApiError };
