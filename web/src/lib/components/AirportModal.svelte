<script lang="ts">
	import { X, Plane, MapPin, ExternalLink, Info } from '@lucide/svelte';
	import type { Airport, RunwayEnd } from '$lib/types';

	// Props
	let { showModal = $bindable(), selectedAirport = $bindable() } = $props<{
		showModal: boolean;
		selectedAirport: Airport | null;
	}>();

	function closeModal() {
		showModal = false;
		selectedAirport = null;
	}

	function formatCoordinates(lat: string | null, lng: string | null): string {
		if (!lat || !lng) return 'Unknown';
		const latNum = parseFloat(lat);
		const lngNum = parseFloat(lng);
		const latDir = latNum >= 0 ? 'N' : 'S';
		const lngDir = lngNum >= 0 ? 'E' : 'W';
		return `${Math.abs(latNum).toFixed(4)}°${latDir}, ${Math.abs(lngNum).toFixed(4)}°${lngDir}`;
	}

	function formatElevation(elevationFt: number | null): string {
		if (elevationFt === null) return 'Unknown';
		return `${elevationFt.toLocaleString()} ft`;
	}

	function formatRunwayLength(lengthFt: number | null): string {
		if (lengthFt === null) return 'Unknown';
		return `${lengthFt.toLocaleString()} ft`;
	}

	function formatRunwayWidth(widthFt: number | null): string {
		if (widthFt === null) return 'Unknown';
		return `${widthFt} ft`;
	}

	function formatHeading(headingDegt: number | null): string {
		if (headingDegt === null) return 'Unknown';
		return `${Math.round(headingDegt)}°`;
	}

	function getAirportTypeDisplay(type: string): string {
		return type
			.split('_')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
			.join(' ');
	}

	function formatRunwayEnds(low: RunwayEnd, high: RunwayEnd): string {
		const lowIdent = low.ident || '?';
		const highIdent = high.ident || '?';
		return `${lowIdent}/${highIdent}`;
	}
</script>

<!-- Airport Modal -->
{#if showModal && selectedAirport}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-surface-950-50/50 pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		role="presentation"
	>
		<div
			class="max-h-[calc(90vh-5rem)] w-full max-w-4xl overflow-y-auto card bg-surface-50 text-surface-900 shadow-xl dark:bg-surface-900 dark:text-surface-50"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="airport-modal-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between border-b border-surface-300 p-6 dark:border-surface-600"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex h-10 w-10 items-center justify-center rounded-full bg-blue-500 text-white"
					>
						<Plane size={24} />
					</div>
					<div>
						<div class="flex items-center gap-2">
							<h2 id="airport-modal-title" class="text-xl font-bold">{selectedAirport.name}</h2>
							<a
								href={`/airports/${selectedAirport.id}`}
								target="_blank"
								rel="noopener noreferrer"
								class="preset-tonal-primary-500 btn btn-sm"
								title="View full airport details"
							>
								<ExternalLink class="h-4 w-4" />
								Details
							</a>
						</div>
						<p class="text-sm text-surface-600 dark:text-surface-400">
							{selectedAirport.ident}
							{#if selectedAirport.municipality}
								• {selectedAirport.municipality}
							{/if}
						</p>
					</div>
				</div>
				<button class="preset-tonal-surface-500 btn btn-sm" onclick={closeModal}>
					<X size={20} />
				</button>
			</div>

			<div class="p-6">
				<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
					<!-- Airport Information -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Info size={20} />
							Airport Information
						</h3>

						<div class="space-y-3">
							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">Type</dt>
									<dd class="text-sm">{getAirportTypeDisplay(selectedAirport.airportType)}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Elevation
									</dt>
									<dd class="text-sm">{formatElevation(selectedAirport.elevationFt)}</dd>
								</div>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										ICAO Code
									</dt>
									<dd class="font-mono text-sm">{selectedAirport.icaoCode || 'N/A'}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										IATA Code
									</dt>
									<dd class="font-mono text-sm">{selectedAirport.iataCode || 'N/A'}</dd>
								</div>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										GPS Code
									</dt>
									<dd class="font-mono text-sm">{selectedAirport.gpsCode || 'N/A'}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Local Code
									</dt>
									<dd class="font-mono text-sm">{selectedAirport.localCode || 'N/A'}</dd>
								</div>
							</div>

							<div>
								<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
									Coordinates
								</dt>
								<dd class="font-mono text-sm">
									{formatCoordinates(selectedAirport.latitudeDeg, selectedAirport.longitudeDeg)}
								</dd>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Country
									</dt>
									<dd class="text-sm">{selectedAirport.isoCountry || 'Unknown'}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">Region</dt>
									<dd class="text-sm">{selectedAirport.isoRegion || 'Unknown'}</dd>
								</div>
							</div>

							<div>
								<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
									Scheduled Service
								</dt>
								<dd class="text-sm">
									<span
										class="badge preset-filled-{selectedAirport.scheduledService
											? 'success'
											: 'secondary'}"
									>
										{selectedAirport.scheduledService ? 'Yes' : 'No'}
									</span>
								</dd>
							</div>

							<!-- Links -->
							{#if selectedAirport.homeLink || selectedAirport.wikipediaLink}
								<div class="space-y-2 border-t border-surface-300 pt-4 dark:border-surface-600">
									<h4 class="text-sm font-medium text-surface-900 dark:text-surface-100">Links</h4>
									<div class="flex flex-col gap-2">
										{#if selectedAirport.homeLink}
											<a
												href={selectedAirport.homeLink}
												target="_blank"
												rel="noopener noreferrer"
												class="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-800 hover:underline dark:text-blue-400 dark:hover:text-blue-300"
											>
												<ExternalLink size={14} />
												Airport Website
											</a>
										{/if}
										{#if selectedAirport.wikipediaLink}
											<a
												href={selectedAirport.wikipediaLink}
												target="_blank"
												rel="noopener noreferrer"
												class="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-800 hover:underline dark:text-blue-400 dark:hover:text-blue-300"
											>
												<ExternalLink size={14} />
												Wikipedia
											</a>
										{/if}
									</div>
								</div>
							{/if}
						</div>
					</div>

					<!-- Runways -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<MapPin size={20} />
							Runways
							<span class="text-sm font-normal text-surface-600 dark:text-surface-400">
								({selectedAirport.runways.length})
							</span>
						</h3>

						{#if selectedAirport.runways.length === 0}
							<div class="py-8 text-center text-surface-500 dark:text-surface-500">
								<MapPin size={48} class="mx-auto mb-2 opacity-50" />
								<p>No runway information available</p>
							</div>
						{:else}
							<div class="space-y-3">
								{#each selectedAirport.runways as runway (runway.id)}
									<div
										class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
									>
										<div class="mb-3 flex items-center justify-between">
											<h4 class="font-mono font-semibold">
												{formatRunwayEnds(runway.low, runway.high)}
											</h4>
											<div class="flex gap-2">
												{#if runway.lighted}
													<span class="badge preset-filled-success-500">
														<svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
															<path
																d="M11 3a1 1 0 10-2 0v1a1 1 0 102 0V3zM15.657 5.757a1 1 0 00-1.414-1.414l-.707.707a1 1 0 001.414 1.414l.707-.707zM18 10a1 1 0 01-1 1h-1a1 1 0 110-2h1a1 1 0 011 1zM5.05 6.464A1 1 0 106.464 5.05l-.707-.707a1 1 0 00-1.414 1.414l.707.707zM5 10a1 1 0 01-1 1H3a1 1 0 110-2h1a1 1 0 011 1zM8 16v-1h4v1a2 2 0 11-4 0zM12 14c.015-.34.208-.646.477-.859a4 4 0 10-4.954 0c.27.213.462.519.476.859h4.002z"
															/>
														</svg>
														Lighted
													</span>
												{/if}
												{#if runway.closed}
													<span class="badge preset-filled-error-500">
														<svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
															<path
																fill-rule="evenodd"
																d="M13.477 14.89A6 6 0 015.11 6.524l8.367 8.368zm1.414-1.414L6.524 5.11a6 6 0 018.367 8.367zM18 10a8 8 0 11-16 0 8 8 0 0116 0z"
																clip-rule="evenodd"
															/>
														</svg>
														Closed
													</span>
												{/if}
											</div>
										</div>

										<!-- Runway Details -->
										<!-- Desktop: Table -->
										<div class="mb-3 hidden md:block">
											<div class="table-container">
												<table class="table-compact table-hover table">
													<tbody>
														<tr>
															<td class="w-1/3 font-medium text-surface-600 dark:text-surface-400"
																>Length</td
															>
															<td>{formatRunwayLength(runway.lengthFt)}</td>
														</tr>
														<tr>
															<td class="w-1/3 font-medium text-surface-600 dark:text-surface-400"
																>Width</td
															>
															<td>{formatRunwayWidth(runway.widthFt)}</td>
														</tr>
														<tr>
															<td class="w-1/3 font-medium text-surface-600 dark:text-surface-400"
																>Surface</td
															>
															<td>{runway.surface || 'Unknown'}</td>
														</tr>
													</tbody>
												</table>
											</div>
										</div>

										<!-- Mobile: Definition List -->
										<dl class="mb-3 space-y-2 text-sm md:hidden">
											<div class="flex justify-between gap-4">
												<dt class="font-medium text-surface-600 dark:text-surface-400">Length</dt>
												<dd class="font-semibold">{formatRunwayLength(runway.lengthFt)}</dd>
											</div>
											<div class="flex justify-between gap-4">
												<dt class="font-medium text-surface-600 dark:text-surface-400">Width</dt>
												<dd class="font-semibold">{formatRunwayWidth(runway.widthFt)}</dd>
											</div>
											<div class="flex justify-between gap-4">
												<dt class="font-medium text-surface-600 dark:text-surface-400">Surface</dt>
												<dd class="font-semibold">{runway.surface || 'Unknown'}</dd>
											</div>
										</dl>

										<!-- Runway End Details -->
										{#if runway.low.headingDegt !== null || runway.high.headingDegt !== null || runway.low.displacedThresholdFt || runway.high.displacedThresholdFt}
											<div class="grid grid-cols-1 gap-3 md:grid-cols-2">
												<!-- Low End -->
												<div>
													<h5 class="mb-1 text-xs font-semibold text-blue-600">
														{runway.low.ident || 'Low'}
													</h5>

													<!-- Desktop: Table -->
													<div class="hidden md:block">
														<div class="table-container">
															<table class="table-compact table">
																<tbody>
																	{#if runway.low.headingDegt !== null}
																		<tr>
																			<td class="text-xs text-surface-600 dark:text-surface-400"
																				>True Hdg</td
																			>
																			<td class="text-xs font-medium"
																				>{formatHeading(runway.low.headingDegt)}</td
																			>
																		</tr>
																	{/if}
																	{#if runway.low.displacedThresholdFt}
																		<tr>
																			<td class="text-xs text-surface-600 dark:text-surface-400"
																				>Displaced</td
																			>
																			<td class="text-xs font-medium"
																				>{runway.low.displacedThresholdFt} ft</td
																			>
																		</tr>
																	{/if}
																</tbody>
															</table>
														</div>
													</div>

													<!-- Mobile: Definition List -->
													<dl class="space-y-1 text-xs md:hidden">
														{#if runway.low.headingDegt !== null}
															<div class="flex justify-between gap-2">
																<dt class="text-surface-600 dark:text-surface-400">True Heading</dt>
																<dd class="font-medium">
																	{formatHeading(runway.low.headingDegt)}
																</dd>
															</div>
														{/if}
														{#if runway.low.displacedThresholdFt}
															<div class="flex justify-between gap-2">
																<dt class="text-surface-600 dark:text-surface-400">Displaced</dt>
																<dd class="font-medium">{runway.low.displacedThresholdFt} ft</dd>
															</div>
														{/if}
													</dl>
												</div>

												<!-- High End -->
												<div>
													<h5 class="mb-1 text-xs font-semibold text-blue-600">
														{runway.high.ident || 'High'}
													</h5>

													<!-- Desktop: Table -->
													<div class="hidden md:block">
														<div class="table-container">
															<table class="table-compact table">
																<tbody>
																	{#if runway.high.headingDegt !== null}
																		<tr>
																			<td class="text-xs text-surface-600 dark:text-surface-400"
																				>True Hdg</td
																			>
																			<td class="text-xs font-medium"
																				>{formatHeading(runway.high.headingDegt)}</td
																			>
																		</tr>
																	{/if}
																	{#if runway.high.displacedThresholdFt}
																		<tr>
																			<td class="text-xs text-surface-600 dark:text-surface-400"
																				>Displaced</td
																			>
																			<td class="text-xs font-medium"
																				>{runway.high.displacedThresholdFt} ft</td
																			>
																		</tr>
																	{/if}
																</tbody>
															</table>
														</div>
													</div>

													<!-- Mobile: Definition List -->
													<dl class="space-y-1 text-xs md:hidden">
														{#if runway.high.headingDegt !== null}
															<div class="flex justify-between gap-2">
																<dt class="text-surface-600 dark:text-surface-400">True Heading</dt>
																<dd class="font-medium">
																	{formatHeading(runway.high.headingDegt)}
																</dd>
															</div>
														{/if}
														{#if runway.high.displacedThresholdFt}
															<div class="flex justify-between gap-2">
																<dt class="text-surface-600 dark:text-surface-400">Displaced</dt>
																<dd class="font-medium">{runway.high.displacedThresholdFt} ft</dd>
															</div>
														{/if}
													</dl>
												</div>
											</div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
