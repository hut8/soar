<script lang="ts">
	import { BarChart3, Activity, Radio } from '@lucide/svelte';
	import { getGrafanaUrl, isStaging } from '$lib/config';

	// Generate metrics with environment-specific Grafana URLs
	const grafanaBase = $derived(getGrafanaUrl());
	const staging = $derived(isStaging());

	const metrics = $derived([
		{
			title: 'Web Performance',
			url: `${grafanaBase}/public-dashboards/${staging ? '9c476a10e31f48bf9a9e0edd13f4285c' : '3ba212e4a59f44a48669e546ca62389b'}`,
			icon: BarChart3,
			description: 'HTTP request metrics, WebSocket connections, and endpoint performance'
		},
		{
			title: 'Processor Performance',
			url: `${grafanaBase}/public-dashboards/${staging ? '15160e9b72244eab82a2031e16d17a97' : 'cbc0efd7f06f45a38815503dca1ff356'}`,
			icon: Activity,
			description: 'APRS message processing, elevation lookups, and flight tracking metrics'
		},
		{
			title: 'APRS Ingest Performance',
			url: `${grafanaBase}/public-dashboards/${staging ? '6ad16fc10c5941b5b09a0ff086309bd8' : 'c3f6fb97f4c04001a04e6f85394a6164'}`,
			icon: Radio,
			description: 'OGN APRS-IS connection status and message publishing metrics'
		}
	]);
</script>

<svelte:head>
	<title>System Info - Glider Flights</title>
	<meta
		name="description"
		content="Learn about the Glider Flights architecture and view real-time performance metrics"
	/>
</svelte:head>

<div class="container mx-auto max-w-6xl space-y-8 p-6">
	<!-- Page Header -->
	<div class="space-y-2">
		<h1 class="text-4xl font-bold">System Information</h1>
		<p class="text-surface-600-300-token text-lg">
			Real-time performance metrics and system architecture
		</p>
	</div>

	<!-- Metrics Section -->
	<section class="space-y-4">
		<h2 class="text-2xl font-semibold">Metrics</h2>
		<div class="grid gap-4 md:grid-cols-2">
			{#each metrics as metric (metric.url)}
				<a
					href={metric.url}
					target="_blank"
					rel="noopener noreferrer"
					class="preset-tonal-primary-500 group flex items-start gap-4 card p-6 transition-all duration-200 hover:scale-[1.02] hover:preset-filled-primary-500"
				>
					<div class="flex-shrink-0 pt-1">
						<svelte:component
							this={metric.icon}
							size={24}
							class="text-primary-500 group-hover:text-white"
						/>
					</div>
					<div class="flex-1 space-y-1">
						<h3 class="text-lg font-semibold">{metric.title}</h3>
						<p class="text-surface-600-300-token text-sm group-hover:text-white/80">
							{metric.description}
						</p>
					</div>
					<div class="flex-shrink-0">
						<svg
							xmlns="http://www.w3.org/2000/svg"
							width="20"
							height="20"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							class="text-surface-400 group-hover:text-white"
						>
							<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
							<polyline points="15 3 21 3 21 9"></polyline>
							<line x1="10" y1="14" x2="21" y2="3"></line>
						</svg>
					</div>
				</a>
			{/each}
		</div>
	</section>

	<!-- About Section -->
	<section class="space-y-4">
		<h2 class="text-2xl font-semibold">About</h2>
		<div class="preset-filled-surface-50-900 space-y-6 card p-6">
			<div class="space-y-3">
				<h3 class="text-xl font-semibold">System Architecture</h3>
				<p class="text-surface-600-300-token leading-relaxed">
					Glider Flights is a real-time aircraft tracking and soaring club management platform built
					with modern, scalable technologies. The system processes APRS (Automatic Packet Reporting
					System) data from the Open Glider Network to track gliders and provide live flight
					information.
				</p>
			</div>

			<div class="space-y-3">
				<h4 class="text-lg font-semibold">Data Flow</h4>
				<ol class="text-surface-600-300-token list-inside list-decimal space-y-2">
					<li>
						<strong>APRS Ingest:</strong> Connects to OGN APRS-IS servers and receives real-time position
						reports from aircraft equipped with APRS transmitters
					</li>
					<li>
						<strong>Processing:</strong> Rust-based processor decodes APRS packets, enriches data with
						elevation information, and maintains flight state
					</li>
					<li>
						<strong>Real-time Distribution:</strong> WebSocket connections push live updates to connected
						clients for instant map and data updates
					</li>
				</ol>
			</div>

			<div class="space-y-3">
				<h4 class="text-lg font-semibold">Technology Stack</h4>
				<div class="grid gap-4 md:grid-cols-2">
					<div>
						<p class="mb-2 font-medium">Backend</p>
						<ul class="text-surface-600-300-token space-y-1 text-sm">
							<li>• Rust with Axum web framework</li>
							<li>• PostgreSQL with PostGIS for spatial data</li>
							<li>• Prometheus for metrics collection</li>
						</ul>
					</div>
					<div>
						<p class="mb-2 font-medium">Frontend</p>
						<ul class="text-surface-600-300-token space-y-1 text-sm">
							<li>• SvelteKit with TypeScript</li>
							<li>• Tailwind CSS + Skeleton UI</li>
							<li>• Leaflet for interactive maps</li>
							<li>• WebSocket for real-time updates</li>
						</ul>
					</div>
				</div>
			</div>

			<div class="space-y-3">
				<h4 class="text-lg font-semibold">Data Sources</h4>
				<ul class="text-surface-600-300-token space-y-2">
					<li>
						<strong
							><a
								href="https://www.glidernet.org/"
								target="_blank"
								rel="noopener noreferrer"
								class="text-primary-500 hover:underline">OGN APRS-IS</a
							>:</strong
						>
						Real-time position reports from gliders and other aircraft via the Open Glider Network (<a
							href="http://wiki.glidernet.org/"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline">documentation</a
						>)
					</li>
					<li>
						<strong
							><a
								href="https://davidmegginson.github.io/ourairports-data/"
								target="_blank"
								rel="noopener noreferrer"
								class="text-primary-500 hover:underline">OurAirports</a
							>:</strong
						>
						Comprehensive airport and runway data (<a
							href="https://davidmegginson.github.io/ourairports-data/airports.csv"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline">airports.csv</a
						>,
						<a
							href="https://davidmegginson.github.io/ourairports-data/runways.csv"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline">runways.csv</a
						>)
					</li>
					<li>
						<strong
							><a
								href="https://registry.faa.gov/"
								target="_blank"
								rel="noopener noreferrer"
								class="text-primary-500 hover:underline">FAA Aircraft Registry</a
							>:</strong
						>
						Aircraft registration data including N-numbers and ownership (<a
							href="https://registry.faa.gov/database/ReleasableAircraft.zip"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline">ReleasableAircraft.zip</a
						>)
					</li>
					<li>
						<strong
							><a
								href="https://turbo87.github.io/united-flarmnet/"
								target="_blank"
								rel="noopener noreferrer"
								class="text-primary-500 hover:underline">United FlarmNet</a
							>:</strong
						>
						Unified FlarmNet database for aircraft identification (<a
							href="https://turbo87.github.io/united-flarmnet/united.fln"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline">united.fln</a
						>)
					</li>
					<li>
						<strong
							><a
								href="https://registry.opendata.aws/terrain-tiles/"
								target="_blank"
								rel="noopener noreferrer"
								class="text-primary-500 hover:underline">Terrain Tiles</a
							>:</strong
						>
						High-resolution digital elevation model from Mapzen for calculating altitude above ground
						level (AGL)
					</li>
				</ul>
			</div>

			<div class="space-y-3">
				<h4 class="text-lg font-semibold">Data License & Attribution</h4>
				<div class="text-surface-600-300-token space-y-2">
					<p>
						Aircraft tracking data provided by the
						<a
							href="https://www.glidernet.org/"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline"><strong>Open Glider Network (OGN)</strong></a
						>, a worldwide network of ground stations tracking aircraft equipped with FLARM and OGN
						trackers.
					</p>
					<p>
						Airport data from OurAirports is licensed under the
						<a
							href="https://opendatacommons.org/licenses/odbl/summary/"
							target="_blank"
							rel="noopener noreferrer"
							class="text-primary-500 hover:underline"
							><strong>Open Database License (ODbL)</strong></a
						>. You are free to copy, distribute, and adapt the data, as long as you credit
						OurAirports and license your adapted database under ODbL.
					</p>
				</div>
			</div>

			<div class="border-surface-200-700-token mt-6 border-t pt-6">
				<p class="text-surface-600-300-token text-sm">
					All metrics dashboards are publicly accessible and update in real-time. The system is
					designed for high availability and processes thousands of position reports per minute with
					sub-second latency.
				</p>
			</div>
		</div>
	</section>
</div>
