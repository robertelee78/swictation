import { useEffect, useRef } from 'react';

interface DataPoint {
  timestamp_unix: number;
  average_wpm: number;
  session_count: number;
}

interface LineChartProps {
  data: DataPoint[];
  width?: number;
  height?: number;
  color?: string;
  showGrid?: boolean;
}

export function LineChart({
  data,
  width = 800,
  height = 300,
  color = '#00ff41',
  showGrid = true
}: LineChartProps) {
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

    // Calculate scales
    const padding = { top: 20, right: 20, bottom: 40, left: 60 };
    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;

    const minWpm = Math.min(...data.map(d => d.average_wpm));
    const maxWpm = Math.max(...data.map(d => d.average_wpm));
    const wpmRange = maxWpm - minWpm || 1;

    const minTime = data[0].timestamp_unix;
    const maxTime = data[data.length - 1].timestamp_unix;
    const timeRange = maxTime - minTime || 1;

    // Draw grid
    if (showGrid) {
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 0.5;

      // Horizontal grid lines (WPM)
      for (let i = 0; i <= 5; i++) {
        const y = padding.top + (chartHeight * i) / 5;
        ctx.beginPath();
        ctx.moveTo(padding.left, y);
        ctx.lineTo(padding.left + chartWidth, y);
        ctx.stroke();

        // WPM labels
        const wpm = maxWpm - (wpmRange * i) / 5;
        ctx.fillStyle = '#888';
        ctx.font = '11px monospace';
        ctx.textAlign = 'right';
        ctx.fillText(Math.round(wpm).toString(), padding.left - 10, y + 4);
      }

      // Vertical grid lines (time)
      for (let i = 0; i <= 6; i++) {
        const x = padding.left + (chartWidth * i) / 6;
        ctx.beginPath();
        ctx.moveTo(x, padding.top);
        ctx.lineTo(x, padding.top + chartHeight);
        ctx.stroke();

        // Time labels
        if (i < 6) {
          const timestamp = minTime + (timeRange * i) / 6;
          const date = new Date(timestamp * 1000);
          const label = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
          ctx.fillStyle = '#888';
          ctx.font = '11px monospace';
          ctx.textAlign = 'center';
          ctx.fillText(label, x + chartWidth / 12, height - 10);
        }
      }
    }

    // Draw axes
    ctx.strokeStyle = '#666';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(padding.left, padding.top);
    ctx.lineTo(padding.left, padding.top + chartHeight);
    ctx.lineTo(padding.left + chartWidth, padding.top + chartHeight);
    ctx.stroke();

    // Axis labels
    ctx.fillStyle = '#aaa';
    ctx.font = 'bold 12px monospace';
    ctx.textAlign = 'center';
    ctx.fillText('WPM', 20, height / 2);
    ctx.fillText('Date', width / 2, height - 5);

    // Draw line chart
    ctx.strokeStyle = color;
    ctx.lineWidth = 2;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    ctx.beginPath();
    data.forEach((point, i) => {
      const x = padding.left + ((point.timestamp_unix - minTime) / timeRange) * chartWidth;
      const y = padding.top + chartHeight - ((point.average_wpm - minWpm) / wpmRange) * chartHeight;

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();

    // Draw data points
    ctx.fillStyle = color;
    data.forEach((point) => {
      const x = padding.left + ((point.timestamp_unix - minTime) / timeRange) * chartWidth;
      const y = padding.top + chartHeight - ((point.average_wpm - minWpm) / wpmRange) * chartHeight;

      ctx.beginPath();
      ctx.arc(x, y, 3, 0, Math.PI * 2);
      ctx.fill();
    });

    // Draw title
    ctx.fillStyle = '#fff';
    ctx.font = 'bold 14px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('WPM Trend Over Time', padding.left, 15);

  }, [data, width, height, color, showGrid]);

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
