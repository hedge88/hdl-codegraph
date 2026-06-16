(function () {
    'use strict';

    const canvas = document.getElementById('graph-canvas');
    const ctx = canvas.getContext('2d');

    let nodes = [];
    let edges = [];
    let scale = 1;
    let offsetX = 0;
    let offsetY = 0;
    let dragging = false;
    let dragStartX, dragStartY;

    function resize() {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
        draw();
    }

    function draw() {
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.save();
        ctx.translate(offsetX, offsetY);
        ctx.scale(scale, scale);

        // Draw edges
        ctx.strokeStyle = getComputedStyle(document.body).getPropertyValue('--vscode-editor-foreground') || '#888';
        ctx.lineWidth = 1;
        for (const edge of edges) {
            ctx.beginPath();
            ctx.moveTo(edge.x1, edge.y1);
            ctx.lineTo(edge.x2, edge.y2);
            ctx.stroke();
        }

        // Draw nodes
        const nodeRadius = 20;
        for (const node of nodes) {
            ctx.beginPath();
            ctx.arc(node.x, node.y, nodeRadius, 0, Math.PI * 2);
            ctx.fillStyle = '#4a9eff';
            ctx.fill();
            ctx.strokeStyle = '#2a7def';
            ctx.lineWidth = 2;
            ctx.stroke();

            ctx.fillStyle = '#fff';
            ctx.font = '10px sans-serif';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(node.label.substring(0, 8), node.x, node.y);
        }

        ctx.restore();
    }

    // Placeholder data
    nodes = [
        { x: 200, y: 150, label: 'top' },
        { x: 100, y: 350, label: 'sub_a' },
        { x: 300, y: 350, label: 'sub_b' },
    ];
    edges = [
        { x1: 200, y1: 170, x2: 100, y2: 330 },
        { x1: 200, y1: 170, x2: 300, y2: 330 },
    ];

    // Toolbar handlers
    document.getElementById('btn-zoom-in').addEventListener('click', () => {
        scale *= 1.2;
        draw();
    });
    document.getElementById('btn-zoom-out').addEventListener('click', () => {
        scale /= 1.2;
        draw();
    });
    document.getElementById('btn-reset').addEventListener('click', () => {
        scale = 1;
        offsetX = 0;
        offsetY = 0;
        draw();
    });

    // Pan via mouse drag
    canvas.addEventListener('mousedown', (e) => {
        dragging = true;
        dragStartX = e.clientX - offsetX;
        dragStartY = e.clientY - offsetY;
    });
    canvas.addEventListener('mousemove', (e) => {
        if (!dragging) return;
        offsetX = e.clientX - dragStartX;
        offsetY = e.clientY - dragStartY;
        draw();
    });
    canvas.addEventListener('mouseup', () => { dragging = false; });
    canvas.addEventListener('mouseleave', () => { dragging = false; });

    // Scroll to zoom
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        scale *= delta;
        scale = Math.min(Math.max(0.1, scale), 10);
        draw();
    });

    window.addEventListener('resize', resize);
    resize();
})();
