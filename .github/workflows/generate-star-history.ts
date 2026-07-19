// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

interface Padding {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

const repo: string = process.env.REPO || process.env.GITHUB_REPOSITORY || 'r3bl-org/r3bl-open-core';

console.log(`Fetching stargazers for repository: ${repo}...`);

let output = '';
try {
  const cmd = `gh api -H "Accept: application/vnd.github.v3.star+json" --paginate "/repos/${repo}/stargazers"`;
  output = execSync(cmd, { encoding: 'utf-8', maxBuffer: 50 * 1024 * 1024 });
} catch (err) {
  console.error('Error fetching stargazers from GitHub API:', err);
  process.exit(1);
}

const stargazers: Date[] = [];
const regex = /"starred_at"\s*:\s*"([^"]+)"/g;
let match: RegExpExecArray | null;
while ((match = regex.exec(output)) !== null) {
  stargazers.push(new Date(match[1]));
}

stargazers.sort((a, b) => a.getTime() - b.getTime());

console.log(`Found ${stargazers.length} stargazers.`);

if (stargazers.length === 0) {
  console.log('No stargazers found.');
  process.exit(0);
}

const now = new Date();
const dates: Date[] = [...stargazers, now];
const counts: number[] = stargazers.map((_, i) => i + 1);
counts.push(counts[counts.length - 1]);

const width = 800;
const height = 400;
const padding: Padding = { top: 60, right: 40, bottom: 60, left: 65 };

const chartWidth = width - padding.left - padding.right;
const chartHeight = height - padding.top - padding.bottom;

const minDate = dates[0].getTime();
const maxDate = dates[dates.length - 1].getTime();
const minCount = 0;
const maxCount = counts[counts.length - 1];

function getX(date: Date): number {
  if (maxDate === minDate) return padding.left + chartWidth;
  return padding.left + ((date.getTime() - minDate) / (maxDate - minDate)) * chartWidth;
}

function getY(count: number): number {
  if (maxCount === minCount) return padding.top + chartHeight;
  return padding.top + chartHeight - ((count - minCount) / (maxCount - minCount)) * chartHeight;
}

const points: string = dates.map((d, i) => `${getX(d).toFixed(1)},${getY(counts[i]).toFixed(1)}`).join(' ');

const fontFamily = "Iosevka, 'JetBrains Mono', 'Fira Code', 'Cascadia Code', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace";

const yTicks = 5;
let yGridHtml = '';
for (let i = 0; i <= yTicks; i++) {
  const val = Math.round((maxCount / yTicks) * i);
  const y = getY(val);
  yGridHtml += `
    <line x1="${padding.left}" y1="${y.toFixed(1)}" x2="${width - padding.right}" y2="${y.toFixed(1)}" stroke="#21262d" stroke-dasharray="4,4" />
    <text x="${padding.left - 12}" y="${(y + 4).toFixed(1)}" fill="#8b949e" font-size="12" text-anchor="end" font-family="${fontFamily}">${val}</text>
  `;
}

const xTicks = 5;
let xGridHtml = '';
for (let i = 0; i <= xTicks; i++) {
  const t = minDate + ((maxDate - minDate) / xTicks) * i;
  const d = new Date(t);
  const x = getX(d);
  const dateStr = d.toLocaleDateString('en-US', { month: 'short', year: '2-digit' });
  xGridHtml += `
    <line x1="${x.toFixed(1)}" y1="${padding.top}" x2="${x.toFixed(1)}" y2="${height - padding.bottom}" stroke="#21262d" stroke-dasharray="4,4" />
    <text x="${x.toFixed(1)}" y="${(height - padding.bottom + 22).toFixed(1)}" fill="#8b949e" font-size="12" text-anchor="middle" font-family="${fontFamily}">${dateStr}</text>
  `;
}

const firstX = getX(dates[0]).toFixed(1);
const lastX = getX(dates[dates.length - 1]).toFixed(1);
const bottomY = (padding.top + chartHeight).toFixed(1);
const areaPoints = `${firstX},${bottomY} ${points} ${lastX},${bottomY}`;

const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="100%" height="100%" style="background-color: #0d1117; font-family: ${fontFamily};">
  <style>
    .title { fill: #c9d1d9; font-size: 16px; font-weight: 600; font-family: ${fontFamily}; }
    .axis-line { stroke: #30363d; stroke-width: 1; }
    .line-path { fill: none; stroke: #58a6ff; stroke-width: 2.5; stroke-linejoin: round; stroke-linecap: round; }
    .area-path { fill: #58a6ff; fill-opacity: 0.12; }
  </style>

  <text x="${padding.left}" y="32" class="title">${repo} (${maxCount} ★)</text>

  <g id="grid">
    ${yGridHtml}
    ${xGridHtml}
  </g>

  <line x1="${padding.left}" y1="${height - padding.bottom}" x2="${width - padding.right}" y2="${height - padding.bottom}" class="axis-line" />
  <line x1="${padding.left}" y1="${padding.top}" x2="${padding.left}" y2="${height - padding.bottom}" class="axis-line" />

  <polygon points="${areaPoints}" class="area-path" />
  <polyline points="${points}" class="line-path" />
</svg>`;

const outputDir = path.join(process.cwd(), '.github/assets');
fs.mkdirSync(outputDir, { recursive: true });
const outputPath = path.join(outputDir, 'star-history.svg');
fs.writeFileSync(outputPath, svg, 'utf-8');
console.log(`Successfully generated ${outputPath} with ${stargazers.length} stargazers.`);
