import type { User } from '$lib/types';
import { serverCall, ServerError } from './server';

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

// Re-export ServerError as AuthApiError for backwards compatibility
export class AuthApiError extends ServerError {
	constructor(message: string, status: number) {
		super(message, status);
		this.name = 'AuthApiError';
	}
}

export const authApi = {
	async login(credentials: LoginRequest): Promise<LoginResponse> {
		return serverCall<LoginResponse>('/auth/login', {
			method: 'POST',
			body: JSON.stringify(credentials)
		});
	},

	async register(userData: RegisterRequest): Promise<void> {
		await serverCall<void>('/auth/register', {
			method: 'POST',
			body: JSON.stringify(userData)
		});
	},

	async getCurrentUser(token: string): Promise<User> {
		return serverCall<User>('/auth/me', {
			headers: {
				Authorization: `Bearer ${token}`
			}
		});
	},

	async requestPasswordReset(data: PasswordResetRequest): Promise<void> {
		await serverCall<void>('/auth/password-reset/request', {
			method: 'POST',
			body: JSON.stringify(data)
		});
	},

	async confirmPasswordReset(data: PasswordResetConfirm): Promise<void> {
		await serverCall<void>('/auth/password-reset/confirm', {
			method: 'POST',
			body: JSON.stringify(data)
		});
	},

	async verifyEmail(token: string): Promise<LoginResponse> {
		return serverCall<LoginResponse>('/auth/verify-email', {
			method: 'POST',
			body: JSON.stringify({ token })
		});
	}
};
