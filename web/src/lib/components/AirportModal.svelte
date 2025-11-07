<script lang="ts">
	import { X, Plane, MapPin, ExternalLink, Info } from '@lucide/svelte';

	// TypeScript interfaces for airport data (from operations page)
	interface RunwayEndView {
		ident: string | null;
		latitude_deg: number | null;
		longitude_deg: number | null;
		elevation_ft: number | null;
		heading_degt: number | null;
		displaced_threshold_ft: number | null;
	}

	interface RunwayView {
		id: number;
		length_ft: number | null;
		width_ft: number | null;
		surface: string | null;
		lighted: boolean;
		closed: boolean;
		low: RunwayEndView;
		high: RunwayEndView;
	}

	interface AirportView {
		id: number;
		ident: string;
		airport_type: string;
		name: string;
		latitude_deg: string | null;
		longitude_deg: string | null;
		elevation_ft: number | null;
		continent: string | null;
		iso_country: string | null;
		iso_region: string | null;
		municipality: string | null;
		scheduled_service: boolean;
		icao_code: string | null;
		iata_code: string | null;
		gps_code: string | null;
		local_code: string | null;
		home_link: string | null;
		wikipedia_link: string | null;
		keywords: string | null;
		runways: RunwayView[];
	}

	// Props
	let { showModal = $bindable(), selectedAirport = $bindable() } = $props<{
		showModal: boolean;
		selectedAirport: AirportView | null;
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

	function formatElevation(elevation_ft: number | null): string {
		if (elevation_ft === null) return 'Unknown';
		return `${elevation_ft.toLocaleString()} ft`;
	}

	function formatRunwayLength(length_ft: number | null): string {
		if (length_ft === null) return 'Unknown';
		return `${length_ft.toLocaleString()} ft`;
	}

	function formatRunwayWidth(width_ft: number | null): string {
		if (width_ft === null) return 'Unknown';
		return `${width_ft} ft`;
	}

	function formatHeading(heading_degt: number | null): string {
		if (heading_degt === null) return 'Unknown';
		return `${Math.round(heading_degt)}°`;
	}

	function getAirportTypeDisplay(type: string): string {
		return type
			.split('_')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
			.join(' ');
	}

	function formatRunwayEnds(low: RunwayEndView, high: RunwayEndView): string {
		const lowIdent = low.ident || '?';
		const highIdent = high.ident || '?';
		return `${lowIdent}/${highIdent}`;
	}
</script>

<!-- Airport Modal -->
{#if showModal && selectedAirport}
	<div
		class="bg-surface-950-50/50 fixed inset-0 z-50 flex items-start justify-center pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		role="presentation"
	>
		<div
			class="card max-h-[calc(90vh-5rem)] w-full max-w-4xl overflow-y-auto bg-white text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="airport-modal-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-gray-200 p-6">
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
						<p class="text-sm text-gray-600">
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
									<dt class="text-sm font-medium text-gray-600">Type</dt>
									<dd class="text-sm">{getAirportTypeDisplay(selectedAirport.airport_type)}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">Elevation</dt>
									<dd class="text-sm">{formatElevation(selectedAirport.elevation_ft)}</dd>
								</div>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-gray-600">ICAO Code</dt>
									<dd class="font-mono text-sm">{selectedAirport.icao_code || 'N/A'}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">IATA Code</dt>
									<dd class="font-mono text-sm">{selectedAirport.iata_code || 'N/A'}</dd>
								</div>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-gray-600">GPS Code</dt>
									<dd class="font-mono text-sm">{selectedAirport.gps_code || 'N/A'}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">Local Code</dt>
									<dd class="font-mono text-sm">{selectedAirport.local_code || 'N/A'}</dd>
								</div>
							</div>

							<div>
								<dt class="text-sm font-medium text-gray-600">Coordinates</dt>
								<dd class="font-mono text-sm">
									{formatCoordinates(selectedAirport.latitude_deg, selectedAirport.longitude_deg)}
								</dd>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-gray-600">Country</dt>
									<dd class="text-sm">{selectedAirport.iso_country || 'Unknown'}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">Region</dt>
									<dd class="text-sm">{selectedAirport.iso_region || 'Unknown'}</dd>
								</div>
							</div>

							<div>
								<dt class="text-sm font-medium text-gray-600">Scheduled Service</dt>
								<dd class="text-sm">
									<span
										class="badge preset-filled-{selectedAirport.scheduled_service
											? 'success'
											: 'secondary'}"
									>
										{selectedAirport.scheduled_service ? 'Yes' : 'No'}
									</span>
								</dd>
							</div>

							<!-- Links -->
							{#if selectedAirport.home_link || selectedAirport.wikipedia_link}
								<div class="space-y-2 border-t border-gray-200 pt-4">
									<h4 class="text-sm font-medium text-gray-900">Links</h4>
									<div class="flex flex-col gap-2">
										{#if selectedAirport.home_link}
											<a
												href={selectedAirport.home_link}
												target="_blank"
												rel="noopener noreferrer"
												class="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-800 hover:underline"
											>
												<ExternalLink size={14} />
												Airport Website
											</a>
										{/if}
										{#if selectedAirport.wikipedia_link}
											<a
												href={selectedAirport.wikipedia_link}
												target="_blank"
												rel="noopener noreferrer"
												class="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-800 hover:underline"
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
							<span class="text-sm font-normal text-gray-600">
								({selectedAirport.runways.length})
							</span>
						</h3>

						{#if selectedAirport.runways.length === 0}
							<div class="py-8 text-center text-gray-500">
								<MapPin size={48} class="mx-auto mb-2 opacity-50" />
								<p>No runway information available</p>
							</div>
						{:else}
							<div class="space-y-3">
								{#each selectedAirport.runways as runway (runway.id)}
									<div class="rounded-lg border border-gray-200 bg-gray-50 p-4">
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

										<!-- Runway Details Table -->
										<div class="table-container mb-3">
											<table class="table-compact table-hover table">
												<tbody>
													<tr>
														<td class="w-1/3 font-medium text-gray-600">Length</td>
														<td>{formatRunwayLength(runway.length_ft)}</td>
													</tr>
													<tr>
														<td class="w-1/3 font-medium text-gray-600">Width</td>
														<td>{formatRunwayWidth(runway.width_ft)}</td>
													</tr>
													<tr>
														<td class="w-1/3 font-medium text-gray-600">Surface</td>
														<td>{runway.surface || 'Unknown'}</td>
													</tr>
												</tbody>
											</table>
										</div>

										<!-- Runway End Details -->
										{#if runway.low.heading_degt !== null || runway.high.heading_degt !== null || runway.low.displaced_threshold_ft || runway.high.displaced_threshold_ft}
											<div class="grid grid-cols-2 gap-3">
												<!-- Low End -->
												<div>
													<h5 class="mb-1 text-xs font-semibold text-blue-600">
														{runway.low.ident || 'Low'}
													</h5>
													<div class="table-container">
														<table class="table-compact table">
															<tbody>
																{#if runway.low.heading_degt !== null}
																	<tr>
																		<td class="text-xs text-gray-600">True Hdg</td>
																		<td class="text-xs font-medium"
																			>{formatHeading(runway.low.heading_degt)}</td
																		>
																	</tr>
																{/if}
																{#if runway.low.displaced_threshold_ft}
																	<tr>
																		<td class="text-xs text-gray-600">Displaced</td>
																		<td class="text-xs font-medium"
																			>{runway.low.displaced_threshold_ft} ft</td
																		>
																	</tr>
																{/if}
															</tbody>
														</table>
													</div>
												</div>

												<!-- High End -->
												<div>
													<h5 class="mb-1 text-xs font-semibold text-blue-600">
														{runway.high.ident || 'High'}
													</h5>
													<div class="table-container">
														<table class="table-compact table">
															<tbody>
																{#if runway.high.heading_degt !== null}
																	<tr>
																		<td class="text-xs text-gray-600">True Hdg</td>
																		<td class="text-xs font-medium"
																			>{formatHeading(runway.high.heading_degt)}</td
																		>
																	</tr>
																{/if}
																{#if runway.high.displaced_threshold_ft}
																	<tr>
																		<td class="text-xs text-gray-600">Displaced</td>
																		<td class="text-xs font-medium"
																			>{runway.high.displaced_threshold_ft} ft</td
																		>
																	</tr>
																{/if}
															</tbody>
														</table>
													</div>
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
