import { serverCall } from '$lib/api/server';
import type {
	PushSubscriptionView,
	VapidPublicKeyResponse,
	DataResponse,
	DataListResponse
} from '$lib/types';

export function isPushSupported(): boolean {
	return 'serviceWorker' in navigator && 'PushManager' in window && 'Notification' in window;
}

export function getPermissionState(): NotificationPermission | 'unsupported' {
	if (!isPushSupported()) return 'unsupported';
	return Notification.permission;
}

export async function getVapidPublicKey(): Promise<string | null> {
	try {
		const response = await serverCall<DataResponse<VapidPublicKeyResponse>>('/push/vapid-key');
		return response.data.publicKey;
	} catch {
		return null;
	}
}

function urlBase64ToUint8Array(base64String: string): Uint8Array {
	const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
	const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
	const rawData = window.atob(base64);
	const outputArray = new Uint8Array(rawData.length);
	for (let i = 0; i < rawData.length; ++i) {
		outputArray[i] = rawData.charCodeAt(i);
	}
	return outputArray;
}

export async function subscribeToPush(): Promise<PushSubscriptionView | null> {
	if (!isPushSupported()) return null;

	const permission = await Notification.requestPermission();
	if (permission !== 'granted') return null;

	const vapidKey = await getVapidPublicKey();
	if (!vapidKey) return null;

	const registration = await navigator.serviceWorker.ready;
	const applicationServerKey = urlBase64ToUint8Array(vapidKey);
	const subscription = await registration.pushManager.subscribe({
		userVisibleOnly: true,
		applicationServerKey: applicationServerKey.buffer as ArrayBuffer
	});

	const json = subscription.toJSON();
	if (!json.endpoint || !json.keys) return null;

	const response = await serverCall<DataResponse<PushSubscriptionView>>('/push/subscribe', {
		method: 'POST',
		body: JSON.stringify({
			endpoint: json.endpoint,
			p256dhKey: json.keys.p256dh,
			authKey: json.keys.auth,
			notifyTakeoff: true,
			notifyLanding: true
		})
	});

	return response.data;
}

export async function unsubscribeFromPush(): Promise<void> {
	if (!isPushSupported()) return;

	const registration = await navigator.serviceWorker.ready;
	const subscription = await registration.pushManager.getSubscription();
	if (subscription) {
		await subscription.unsubscribe();
	}
}

export async function getSubscriptions(): Promise<PushSubscriptionView[]> {
	try {
		const response =
			await serverCall<DataListResponse<PushSubscriptionView>>('/push/subscriptions');
		return response.data;
	} catch {
		return [];
	}
}

export async function deleteSubscription(id: string): Promise<void> {
	await serverCall(`/push/subscriptions/${id}`, { method: 'DELETE' });
}

export async function isCurrentlySubscribed(): Promise<boolean> {
	if (!isPushSupported()) return false;
	try {
		const registration = await navigator.serviceWorker.ready;
		const subscription = await registration.pushManager.getSubscription();
		return subscription !== null;
	} catch {
		return false;
	}
}
