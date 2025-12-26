#!/usr/bin/env node

/**
 * Reorganize Grafana dashboard panels to full width and vertical stack
 */

const fs = require('fs');
const path = require('path');

const dashboardPath = process.argv[2];

if (!dashboardPath) {
  console.error('Usage: node reorganize-dashboard.js <dashboard-file.json>');
  process.exit(1);
}

// Read the dashboard
const dashboard = JSON.parse(fs.readFileSync(dashboardPath, 'utf8'));

// Standard panel height
const PANEL_HEIGHT = 8;
const FULL_WIDTH = 24;

let currentY = 0;

// Reorganize all panels
if (dashboard.panels && Array.isArray(dashboard.panels)) {
  dashboard.panels.forEach((panel) => {
    // Set full width and consistent height
    panel.gridPos = {
      x: 0,
      y: currentY,
      w: FULL_WIDTH,
      h: PANEL_HEIGHT
    };

    currentY += PANEL_HEIGHT;

    // Handle row panels (they should also be full width)
    if (panel.type === 'row') {
      // Rows typically have smaller height
      panel.gridPos.h = 1;
      currentY -= PANEL_HEIGHT;
      currentY += 1;
    }
  });

  console.log(`Reorganized ${dashboard.panels.length} panels`);
  console.log(`Total dashboard height: ${currentY} units`);
}

// Write back to file
fs.writeFileSync(dashboardPath, JSON.stringify(dashboard, null, 2));
console.log(`Dashboard saved to ${dashboardPath}`);
