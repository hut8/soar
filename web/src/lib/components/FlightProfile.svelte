<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import Plotly from 'plotly.js-dist-min';
	import { theme } from '$lib/stores/theme';
	import type { Fix } from '$lib/types';

	let {
		fixes,
		hasAglData = false,
		onHover = undefined,
		onUnhover = undefined,
		recreateTrigger = $bindable(0)
	}: {
		fixes: Fix[];
		hasAglData?: boolean;
		onHover?: (fix: Fix) => void;
		onUnhover?: () => void;
		recreateTrigger?: number;
	} = $props();

	let chartContainer = $state<HTMLElement>();
	let chartInitialized = $state(false);

	// Store event listener functions so we can remove them
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let hoverListener: ((event: any) => void) | null = null;
	let unhoverListener: (() => void) | null = null;

	// Get Plotly layout configuration based on theme
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function getPlotlyLayout(isDark: boolean): any {
		return {
			xaxis: {
				title: {
					text: 'Time',
					font: {
						color: isDark ? '#e5e7eb' : '#111827'
					}
				},
				type: 'date',
				color: isDark ? '#9ca3af' : '#6b7280',
				gridcolor: isDark ? '#374151' : '#e5e7eb'
			},
			yaxis: {
				title: {
					text: 'Altitude (ft)',
					font: {
						color: isDark ? '#e5e7eb' : '#111827'
					}
				},
				rangemode: 'tozero',
				color: isDark ? '#9ca3af' : '#6b7280',
				gridcolor: isDark ? '#374151' : '#e5e7eb'
			},
			yaxis2: {
				title: {
					text: 'Ground Speed (kt)',
					font: {
						color: isDark ? '#e5e7eb' : '#111827'
					}
				},
				overlaying: 'y',
				side: 'right',
				rangemode: 'tozero',
				color: isDark ? '#9ca3af' : '#6b7280',
				showgrid: false
			},
			hovermode: 'x unified',
			showlegend: true,
			legend: {
				x: 0.01,
				y: 0.99,
				bgcolor: isDark ? 'rgba(31, 41, 55, 0.8)' : 'rgba(255, 255, 255, 0.8)',
				font: {
					color: isDark ? '#e5e7eb' : '#111827'
				}
			},
			margin: { l: 60, r: 60, t: 40, b: 60 },
			paper_bgcolor: isDark ? '#1f2937' : '#ffffff',
			plot_bgcolor: isDark ? '#111827' : '#f9fafb'
		};
	}

	async function createChart() {
		if (!chartContainer || fixes.length === 0) return;

		try {
			// Remove old event listeners if they exist
			if (hoverListener && chartContainer) {
				chartContainer.removeEventListener('plotly_hover', hoverListener);
			}
			if (unhoverListener && chartContainer) {
				chartContainer.removeEventListener('plotly_unhover', unhoverListener);
			}

			const fixesInOrder = [...fixes].reverse();
			const timestamps = fixesInOrder.map((fix) => new Date(fix.timestamp));
			const altitudesMsl = fixesInOrder.map((fix) => fix.altitude_msl_feet || 0);
			const groundSpeeds = fixesInOrder.map((fix) => fix.ground_speed_knots || 0);

			const traces = [
				{
					x: timestamps,
					y: altitudesMsl,
					type: 'scatter' as const,
					mode: 'lines' as const,
					name: 'MSL Altitude',
					line: { color: '#3b82f6', width: 2 },
					hovertemplate: '<b>MSL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
				}
			];

			if (hasAglData) {
				const altitudesAgl = fixesInOrder.map((fix) => fix.altitude_agl_feet || 0);
				traces.push({
					x: timestamps,
					y: altitudesAgl,
					type: 'scatter' as const,
					mode: 'lines' as const,
					name: 'AGL Altitude',
					line: { color: '#10b981', width: 2 },
					hovertemplate: '<b>AGL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
				});
			}

			// Add ground speed trace on secondary y-axis
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const groundSpeedTrace: any = {
				x: timestamps,
				y: groundSpeeds,
				type: 'scatter' as const,
				mode: 'lines' as const,
				name: 'Ground Speed',
				line: { color: '#f59e0b', width: 2 },
				yaxis: 'y2',
				hovertemplate: '<b>GS:</b> %{y:.0f} kt<br>%{x}<extra></extra>'
			};
			traces.push(groundSpeedTrace);

			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const layout: any = getPlotlyLayout($theme === 'dark');
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const config: any = {
				responsive: true,
				displayModeBar: true,
				displaylogo: false,
				modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d']
			};

			await Plotly.newPlot(chartContainer, traces, layout, config);
			chartInitialized = true;

			// Add hover event handlers if callbacks are provided
			if (onHover) {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				hoverListener = (event: any) => {
					const data_event = event.detail || event;
					if (data_event.points && data_event.points.length > 0) {
						const pointIndex = data_event.points[0].pointIndex;
						if (pointIndex >= 0 && pointIndex < fixesInOrder.length) {
							const fix = fixesInOrder[pointIndex];
							onHover(fix);
						}
					}
				};
				chartContainer.addEventListener('plotly_hover', hoverListener);
			}

			if (onUnhover) {
				unhoverListener = () => {
					onUnhover();
				};
				chartContainer.addEventListener('plotly_unhover', unhoverListener);
			}
		} catch (error) {
			console.error('Failed to create flight profile chart:', error);
		}
	}

	// Initialize chart on mount
	onMount(() => {
		createChart();
	});

	// Update chart when theme changes
	$effect(() => {
		const currentTheme = $theme;

		if (chartInitialized && chartContainer && Plotly && fixes.length > 0) {
			const isDark = currentTheme === 'dark';
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const currentData = (chartContainer as any).data || [];

			if (currentData.length > 0) {
				Plotly.react(chartContainer, currentData, getPlotlyLayout(isDark));
			}
		}
	});

	// Recreate chart when trigger changes (for collapsible panels, etc.)
	$effect(() => {
		// Watch recreateTrigger - when it changes, recreate the chart
		if (recreateTrigger > 0 && chartContainer && fixes.length > 0) {
			// Wait for next tick to ensure DOM is ready
			setTimeout(() => {
				createChart();
			}, 100);
		}
	});

	// Cleanup
	onDestroy(() => {
		// Remove event listeners
		if (hoverListener && chartContainer) {
			chartContainer.removeEventListener('plotly_hover', hoverListener);
		}
		if (unhoverListener && chartContainer) {
			chartContainer.removeEventListener('plotly_unhover', unhoverListener);
		}

		// Purge Plotly chart
		if (chartContainer) {
			Plotly.purge(chartContainer);
		}
	});
</script>

<div bind:this={chartContainer} class="h-full w-full"></div>
