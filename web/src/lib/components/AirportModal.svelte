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
		const typeMap: Record<string, string> = {
			large_airport: 'Large Airport',
			medium_airport: 'Medium Airport',
			small_airport: 'Small Airport',
			seaplane_base: 'Seaplane Base',
			heliport: 'Heliport',
			balloonport: 'Balloonport',
			closed: 'Closed'
		};
		return typeMap[type] || type;
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
		class="fixed inset-0 z-50 flex items-start justify-center bg-surface-950-50/50 pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		tabindex="-1"
		role="dialog"
	>
		<div
			class="max-h-[calc(90vh-5rem)] w-full max-w-4xl overflow-y-auto card bg-white text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			tabindex="0"
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
						<h2 class="text-xl font-bold">{selectedAirport.name}</h2>
						<p class="text-sm text-gray-600">
							{selectedAirport.ident}
							{#if selectedAirport.municipality}
								• {selectedAirport.municipality}
							{/if}
						</p>
					</div>
				</div>
				<button class="variant-ghost-surface btn btn-sm" onclick={closeModal}>
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
										class="badge variant-filled-{selectedAirport.scheduled_service
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
									<div
										class="rounded-lg border border-gray-200 bg-gray-50 p-4"
										class:opacity-50={runway.closed}
									>
										<div class="mb-2 flex items-center justify-between">
											<h4 class="font-mono font-semibold">
												{formatRunwayEnds(runway.low, runway.high)}
											</h4>
											<div class="flex gap-2">
												{#if runway.lighted}
													<span class="variant-filled-success badge text-xs">Lighted</span>
												{/if}
												{#if runway.closed}
													<span class="variant-filled-error badge text-xs">Closed</span>
												{/if}
											</div>
										</div>

										<dl class="grid grid-cols-2 gap-3 text-sm">
											<div>
												<dt class="font-medium text-gray-600">Length</dt>
												<dd>{formatRunwayLength(runway.length_ft)}</dd>
											</div>
											<div>
												<dt class="font-medium text-gray-600">Width</dt>
												<dd>{formatRunwayWidth(runway.width_ft)}</dd>
											</div>
											<div class="col-span-2">
												<dt class="font-medium text-gray-600">Surface</dt>
												<dd>{runway.surface || 'Unknown'}</dd>
											</div>

											<!-- Runway End Details -->
											{#if runway.low.heading_degt !== null || runway.high.heading_degt !== null}
												<div class="col-span-2 border-t border-gray-200 pt-2">
													<dt class="mb-1 font-medium text-gray-600">Headings</dt>
													<dd class="grid grid-cols-2 gap-2 text-xs">
														<div>
															<span class="font-medium">{runway.low.ident || '?'}:</span>
															{formatHeading(runway.low.heading_degt)}
														</div>
														<div>
															<span class="font-medium">{runway.high.ident || '?'}:</span>
															{formatHeading(runway.high.heading_degt)}
														</div>
													</dd>
												</div>
											{/if}

											{#if runway.low.displaced_threshold_ft || runway.high.displaced_threshold_ft}
												<div class="col-span-2">
													<dt class="mb-1 font-medium text-gray-600">Displaced Threshold</dt>
													<dd class="grid grid-cols-2 gap-2 text-xs">
														{#if runway.low.displaced_threshold_ft}
															<div>
																<span class="font-medium">{runway.low.ident || '?'}:</span>
																{runway.low.displaced_threshold_ft} ft
															</div>
														{/if}
														{#if runway.high.displaced_threshold_ft}
															<div>
																<span class="font-medium">{runway.high.ident || '?'}:</span>
																{runway.high.displaced_threshold_ft} ft
															</div>
														{/if}
													</dd>
												</div>
											{/if}
										</dl>
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
