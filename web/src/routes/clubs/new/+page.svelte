<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { Plus, ArrowLeft } from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { toaster } from '$lib/toaster';
	import AirportSelector from '$lib/components/AirportSelector.svelte';
	import type { Club, DataResponse, Airport } from '$lib/types';

	let name = $state('');
	let isSoaring = $state(true);
	let homeBaseAirportId = $state<number | null>(null);
	let airportSelectorValue = $state<string[]>([]);
	let street1 = $state('');
	let street2 = $state('');
	let city = $state('');
	let addressState = $state('');
	let zipCode = $state('');
	let countryCode = $state('');
	let submitting = $state(false);
	let error = $state('');

	function handleAirportSelect(airport: Airport | null) {
		homeBaseAirportId = airport ? airport.id : null;
	}

	async function handleSubmit(event: Event) {
		event.preventDefault();
		error = '';

		if (!name.trim()) {
			error = 'Club name is required.';
			return;
		}

		submitting = true;

		try {
			const body: Record<string, unknown> = {
				name: name.trim(),
				isSoaring
			};

			if (homeBaseAirportId != null) {
				body.homeBaseAirportId = homeBaseAirportId;
			}

			// Include address fields if any are filled
			if (street1.trim()) body.street1 = street1.trim();
			if (street2.trim()) body.street2 = street2.trim();
			if (city.trim()) body.city = city.trim();
			if (addressState.trim()) body.state = addressState.trim();
			if (zipCode.trim()) body.zipCode = zipCode.trim();
			if (countryCode.trim()) body.countryCode = countryCode.trim();

			const response = await serverCall<DataResponse<Club>>('/clubs', {
				method: 'POST',
				body: JSON.stringify(body)
			});

			toaster.success({
				title: 'Club created',
				description: `"${response.data.name}" has been created and is pending approval.`
			});

			goto(resolve(`/clubs/${response.data.id}`));
		} catch (err) {
			if (err instanceof Error) {
				error = err.message;
			} else {
				error = 'Failed to create club. Please try again.';
			}
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>Create Club - SOAR</title>
</svelte:head>

<div class="container mx-auto max-w-2xl space-y-6 p-4">
	<a href={resolve('/clubs')} class="inline-flex items-center gap-1 anchor text-sm">
		<ArrowLeft class="h-4 w-4" />
		Back to Clubs
	</a>

	<header class="space-y-2">
		<h1 class="flex items-center gap-2 h1">
			<Plus class="h-8 w-8" />
			Create a Club
		</h1>
		<p class="text-surface-500-400-token">
			Submit a new club for approval. Once approved, it will appear in club listings.
		</p>
	</header>

	{#if !$auth.isAuthenticated}
		<div class="card p-8 text-center">
			<p class="text-surface-500-400-token">You must be logged in to create a club.</p>
			<a href={resolve('/login')} class="mt-4 btn preset-filled-primary-500">Log In</a>
		</div>
	{:else}
		<form class="space-y-6 card p-6" onsubmit={handleSubmit}>
			{#if error}
				<div
					class="rounded border border-red-200 bg-red-50 p-3 text-sm text-red-600 dark:border-red-800 dark:bg-red-950 dark:text-red-400"
				>
					{error}
				</div>
			{/if}

			<!-- Club Name -->
			<label class="label">
				<span class="font-semibold">Club Name <span class="text-error-500">*</span></span>
				<input
					bind:value={name}
					class="input"
					type="text"
					placeholder="e.g. Mountain Soaring Club"
					required
					disabled={submitting}
				/>
			</label>

			<!-- Is Soaring Club -->
			<label class="flex items-center gap-3">
				<input type="checkbox" bind:checked={isSoaring} class="checkbox" disabled={submitting} />
				<span>This is a soaring/gliding club</span>
			</label>

			<!-- Home Base Airport -->
			<div>
				<span class="label mb-1 block font-semibold">Home Base Airport</span>
				<AirportSelector
					bind:value={airportSelectorValue}
					onSelect={handleAirportSelect}
					placeholder="Search for home base airport..."
					disabled={submitting}
				/>
			</div>

			<!-- Address Section -->
			<fieldset class="space-y-4 rounded-lg border border-surface-300 p-4 dark:border-surface-600">
				<legend class="px-2 text-sm font-semibold text-surface-600 dark:text-surface-400"
					>Address (optional)</legend
				>

				<label class="label">
					<span>Street Address</span>
					<input
						bind:value={street1}
						class="input"
						type="text"
						placeholder="123 Airport Rd"
						disabled={submitting}
					/>
				</label>

				<label class="label">
					<span>Street Address 2</span>
					<input
						bind:value={street2}
						class="input"
						type="text"
						placeholder="Suite 100"
						disabled={submitting}
					/>
				</label>

				<div class="grid grid-cols-2 gap-4">
					<label class="label">
						<span>City</span>
						<input
							bind:value={city}
							class="input"
							type="text"
							placeholder="City"
							disabled={submitting}
						/>
					</label>

					<label class="label">
						<span>State</span>
						<input
							bind:value={addressState}
							class="input"
							type="text"
							placeholder="State"
							disabled={submitting}
						/>
					</label>
				</div>

				<div class="grid grid-cols-2 gap-4">
					<label class="label">
						<span>ZIP Code</span>
						<input
							bind:value={zipCode}
							class="input"
							type="text"
							placeholder="12345"
							disabled={submitting}
						/>
					</label>

					<label class="label">
						<span>Country Code</span>
						<input
							bind:value={countryCode}
							class="input"
							type="text"
							placeholder="US"
							maxlength="2"
							disabled={submitting}
						/>
					</label>
				</div>
			</fieldset>

			<!-- Submit -->
			<button
				type="submit"
				class="btn w-full preset-filled-primary-500"
				disabled={submitting || !name.trim()}
			>
				{#if submitting}
					<Progress class="mr-2 h-4 w-4" />
					Creating...
				{:else}
					<Plus class="mr-2 h-4 w-4" />
					Create Club
				{/if}
			</button>
		</form>
	{/if}
</div>
