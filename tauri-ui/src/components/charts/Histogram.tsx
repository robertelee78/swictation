import { useEffect, useRef } from 'react';

interface HistogramProps {
  data: number[];
  bins?: number;
  width?: number;
  height?: number;
  color?: string;
  label?: string;
}

export function Histogram({
  data,
  bins = 20,
  width = 800,
  height = 300,
  color = '#ffa500',
  label = 'Latency (ms)',
}: HistogramProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || data.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // High DPI support
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Calculate histogram bins
    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    const binWidth = range / bins;

    const binCounts = new Array(bins).fill(0);
    data.forEach((value) => {
      const binIndex = Math.min(Math.floor((value - min) / binWidth), bins - 1);
      binCounts[binIndex]++;
    });

    const maxCount = Math.max(...binCounts);

    // Layout
    const padding = { top: 40, right: 20, bottom: 60, left: 60 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;
    const barWidth = chartWidth / bins;

    // Draw axes
    ctx.strokeStyle = '#666';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(padding.left, padding.top);
    ctx.lineTo(padding.left, padding.top + chartHeight);
    ctx.lineTo(padding.left + chartWidth, padding.top + chartHeight);
    ctx.stroke();

    // Draw bars
    binCounts.forEach((count, i) => {
      const barHeight = (count / maxCount) * chartHeight;
      const x = padding.left + i * barWidth;
      const y = padding.top + chartHeight - barHeight;

      // Gradient for bars
      const gradient = ctx.createLinearGradient(x, y, x, y + barHeight);
      gradient.addColorStop(0, color);
      gradient.addColorStop(1, color + '60');

      ctx.fillStyle = gradient;
      ctx.fillRect(x + 1, y, barWidth - 2, barHeight);

      // Bar outline
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 1, y, barWidth - 2, barHeight);
    });

    // Draw frequency labels (Y-axis)
    ctx.fillStyle = '#888';
    ctx.font = '11px monospace';
    ctx.textAlign = 'right';
    for (let i = 0; i <= 5; i++) {
      const y = padding.top + (chartHeight * i) / 5;
      const freq = Math.round(maxCount - (maxCount * i) / 5);
      ctx.fillText(freq.toString(), padding.left - 10, y + 4);
    }

    // Draw value labels (X-axis) - show min, median, max
    ctx.textAlign = 'center';
    const labelPositions = [0, Math.floor(bins / 2), bins - 1];
    labelPositions.forEach((i) => {
      const x = padding.left + i * barWidth + barWidth / 2;
      const value = min + i * binWidth;
      ctx.fillText(Math.round(value).toString(), x, padding.top + chartHeight + 20);
    });

    // Axis labels
    ctx.fillStyle = '#aaa';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'center';
    ctx.fillText('Frequency', 20, height / 2);
    ctx.fillText(label, width / 2, height - 5);

    // Draw title with stats
    const mean = data.reduce((a, b) => a + b, 0) / data.length;
    const sorted = [...data].sort((a, b) => a - b);
    const median = sorted[Math.floor(sorted.length / 2)];

    ctx.fillStyle = '#fff';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'left';
    ctx.fillText(`${label} Distribution`, padding.left, 15);

    ctx.font = '11px monospace';
    ctx.fillStyle = '#aaa';
    ctx.fillText(
      `Mean: ${Math.round(mean)} | Median: ${Math.round(median)} | Range: ${Math.round(min)}-${Math.round(max)}`,
      padding.left,
      30
    );

  }, [data, bins, width, height, color, label]);

  return (
    <canvas
      ref={canvasRef}
      style={{
        width: `${width}px`,
        height: `${height}px`,
        background: '#0a0a0a',
        borderRadius: '8px',
      }}
    />
  );
}
