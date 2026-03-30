import { useState, useMemo } from "react";

// ── Data ──────────────────────────────────────────────────────────────────────
const HARDWARE = [
  // CPU-only tiers
  { id:"consumer-cpu", cls:"CPU", tier:"Consumer Desktop", chip:"Ryzen 9 9950X / i9-14900K", mem:"DDR5-6400 Dual-Ch", memType:"DDR5", unified:false, coherent:true, bw:80, capacity:128, simd:"AVX-512 / AVX2", note:"2 channels", color:"#5b8def" },
  { id:"hedt-cpu", cls:"CPU", tier:"HEDT / Workstation", chip:"Threadripper 7980X", mem:"DDR5-5600 Quad-Ch", memType:"DDR5", unified:false, coherent:true, bw:180, capacity:512, simd:"AVX-512", note:"4 channels", color:"#4a7ad9" },
  { id:"server-1s-cpu", cls:"CPU", tier:"Server 1-Socket", chip:"EPYC 9555 (64c)", mem:"DDR5-6400 8-Ch", memType:"DDR5", unified:false, coherent:true, bw:310, capacity:1536, simd:"AVX-512 / VNNI", note:"8 channels", color:"#3966c3" },
  { id:"server-2s-cpu", cls:"CPU", tier:"Server 2-Socket", chip:"2× EPYC 9654 (2×96c)", mem:"DDR5-4800 24-Ch", memType:"DDR5", unified:false, coherent:true, bw:460, capacity:6144, simd:"AVX-512 / VNNI", note:"12 ch × 2 sockets", color:"#2952ad" },
  { id:"xeon-amx", cls:"CPU", tier:"Server 2-Socket (AMX)", chip:"2× Xeon 6980P (2×128c)", mem:"MRDIMM-8800 16-Ch", memType:"MRDIMM", unified:false, coherent:true, bw:560, capacity:4096, simd:"AMX / AVX-512", note:"8 ch × 2 sockets, AMX tiles", color:"#1e3f97" },
  // Unified memory (Apple)
  { id:"m4-max", cls:"APU", tier:"Apple M4 Max", chip:"M4 Max (40-core GPU)", mem:"Unified LPDDR5X", memType:"Unified LPDDR5X", unified:true, coherent:true, bw:410, capacity:128, simd:"NEON + AMX", note:"512-bit bus", color:"#e06030" },
  { id:"m4-ultra", cls:"APU", tier:"Apple M4 Ultra", chip:"M4 Ultra (80-core GPU)", mem:"Unified LPDDR5X", memType:"Unified LPDDR5X", unified:true, coherent:true, bw:819, capacity:256, simd:"NEON + AMX", note:"1024-bit bus", color:"#c04820" },
  // NPU / SoC
  { id:"strix-halo", cls:"APU", tier:"AMD Strix Halo", chip:"Ryzen AI Max+ 395", mem:"LPDDR5X-8000", memType:"LPDDR5X", unified:true, coherent:true, bw:256, capacity:128, simd:"AVX-512 + XDNA2 NPU", note:"8-ch LPDDR5X, 50 TOPS NPU", color:"#d4a030" },
  // GPU tiers
  { id:"rtx4090", cls:"GPU", tier:"Consumer GPU", chip:"RTX 4090", mem:"GDDR6X", memType:"GDDR6X", unified:false, coherent:false, bw:1008, capacity:24, simd:"Tensor Cores (FP8)", note:"24 GB VRAM", color:"#30b050" },
  { id:"rtx5090", cls:"GPU", tier:"Consumer GPU (Next)", chip:"RTX 5090", mem:"GDDR7", memType:"GDDR7", unified:false, coherent:false, bw:1792, capacity:32, simd:"Tensor Cores (FP4)", note:"32 GB VRAM", color:"#20a040" },
  { id:"a100", cls:"GPU", tier:"Data Center GPU", chip:"A100 80GB", mem:"HBM2e", memType:"HBM2e", unified:false, coherent:false, bw:2039, capacity:80, simd:"Tensor Cores (FP16)", note:"80 GB HBM2e", color:"#109030" },
  { id:"h100", cls:"GPU", tier:"Data Center GPU", chip:"H100 SXM", mem:"HBM3", memType:"HBM3", unified:false, coherent:false, bw:3350, capacity:80, simd:"Tensor Cores (FP8)", note:"80 GB HBM3", color:"#008020" },
  { id:"h200", cls:"GPU", tier:"Data Center GPU", chip:"H200 SXM", mem:"HBM3e", memType:"HBM3e", unified:false, coherent:false, bw:4800, capacity:141, simd:"Tensor Cores (FP8)", note:"141 GB HBM3e", color:"#006818" },
  { id:"b200", cls:"GPU", tier:"Data Center GPU (Next)", chip:"B200", mem:"HBM3e", memType:"HBM3e", unified:false, coherent:false, bw:8000, capacity:192, simd:"Tensor Cores (FP4)", note:"192 GB HBM3e", color:"#005010" },
  // Multi-GPU
  { id:"4xh100", cls:"Multi-GPU", tier:"4× H100 NVLink", chip:"4× H100 SXM", mem:"HBM3", memType:"HBM3", unified:false, coherent:false, bw:13400, capacity:320, simd:"Tensor Cores (FP8)", note:"NVLink 900 GB/s inter-GPU", color:"#7030a0" },
  { id:"8xh200", cls:"Multi-GPU", tier:"8× H200 NVLink", chip:"8× H200 SXM (DGX)", mem:"HBM3e", memType:"HBM3e", unified:false, coherent:false, bw:38400, capacity:1128, simd:"Tensor Cores (FP8)", note:"DGX H200 node", color:"#5820a0" },
];

const MODELS = [
  { id:"llama-70b", name:"Llama 3.3 70B", params:70, active:70, arch:"Dense", sizeQ4:40, sizeFP16:140, moeExperts:0, moeActive:0 },
  { id:"gptoss-120b", name:"GPT-OSS 120B", params:120, active:120, arch:"Dense", sizeQ4:68, sizeFP16:240, moeExperts:0, moeActive:0 },
  { id:"llama-405b", name:"Llama 3.1 405B", params:405, active:405, arch:"Dense", sizeQ4:230, sizeFP16:810, moeExperts:0, moeActive:0 },
  { id:"ds-r1-671b", name:"DeepSeek R1 671B", params:671, active:37, arch:"MoE (256×8)", sizeQ4:377, sizeFP16:1340, moeExperts:256, moeActive:8 },
];

const QUANTS = [
  { id:"Q4", label:"Q4_K_M (~4-bit)", factor:0.5 },
  { id:"Q8", label:"Q8_0 (~8-bit)", factor:1.0 },
  { id:"FP16", label:"FP16 (16-bit)", factor:2.0 },
];

function estimatePerf(hw, model, quant) {
  const modelBytes = model.params * quant.factor * 1e9 / 1e9; // in GB conceptually
  const modelSizeGB = quant.id === "Q4" ? model.sizeQ4 : quant.id === "Q8" ? (model.sizeQ4 * 2) : model.sizeFP16;
  const activeSizeGB = model.arch.startsWith("MoE") ? modelSizeGB * (model.active / model.params) * 1.15 : modelSizeGB;

  const fits = modelSizeGB <= hw.capacity;
  const activeFits = activeSizeGB <= hw.capacity;

  // Token generation is BW-bound: ~= BW / bytes_read_per_token
  // For dense: read entire model per token
  // For MoE: read active experts + shared layers per token
  const bytesPerToken = model.arch.startsWith("MoE") ? activeSizeGB : modelSizeGB;
  const rawTGtps = hw.bw / bytesPerToken;

  // Efficiency factor: CPU ~60-75%, GPU ~80-90%, Multi-GPU ~70-85% (NVLink overhead)
  let eff = 0.65;
  if (hw.cls === "GPU") eff = 0.85;
  if (hw.cls === "Multi-GPU") eff = 0.78;
  if (hw.cls === "APU") eff = 0.72;
  if (hw.id === "xeon-amx") eff = 0.70;
  if (hw.simd?.includes("AMX") && hw.cls === "CPU") eff = 0.70;

  const tgTps = rawTGtps * eff;

  // Prefill is compute-bound, roughly 3-10x tg for CPU, 10-50x for GPU
  let ppMultiplier = hw.cls === "GPU" || hw.cls === "Multi-GPU" ? 25 : hw.cls === "APU" ? 5 : 4;
  if (hw.simd?.includes("AMX")) ppMultiplier *= 1.5;
  const ppTps = Math.min(tgTps * ppMultiplier, hw.bw / (activeSizeGB * 0.05)); // cap

  return { fits, activeFits, tgTps: Math.round(tgTps * 10) / 10, ppTps: Math.round(ppTps), modelSizeGB, activeSizeGB: Math.round(activeSizeGB) };
}

// ── Components ────────────────────────────────────────────────────────────────

const Tab = ({ active, onClick, children }) => (
  <button onClick={onClick} style={{
    padding:"10px 22px", border:"none", cursor:"pointer", fontSize:13, fontWeight:600,
    letterSpacing:"0.03em", textTransform:"uppercase",
    background: active ? "#1a1a2e" : "transparent",
    color: active ? "#e8d5b7" : "#8a8a9a",
    borderBottom: active ? "2px solid #e8d5b7" : "2px solid transparent",
    transition:"all 0.2s"
  }}>{children}</button>
);

const Badge = ({ children, color }) => (
  <span style={{
    display:"inline-block", padding:"2px 8px", borderRadius:3, fontSize:10, fontWeight:700,
    background: color || "#333", color:"#fff", letterSpacing:"0.05em", marginLeft:4
  }}>{children}</span>
);

const BarChart = ({ value, max, color, label, width = 200 }) => (
  <div style={{ display:"flex", alignItems:"center", gap:8 }}>
    <div style={{ width, height:16, background:"#1a1a2e", borderRadius:2, overflow:"hidden", position:"relative" }}>
      <div style={{
        width: `${Math.min((value / max) * 100, 100)}%`, height:"100%",
        background: `linear-gradient(90deg, ${color}88, ${color})`,
        borderRadius:2, transition:"width 0.4s ease"
      }} />
    </div>
    <span style={{ fontSize:12, color:"#c8c8d0", fontFamily:"'JetBrains Mono', monospace", minWidth:60 }}>{label}</span>
  </div>
);

function MemoryHierarchyDiagram() {
  const tiers = [
    { label:"SRAM / Cache", bw:"~10 TB/s", cap:"< 100 MB", lat:"< 1 ns", color:"#ff4060", w:60 },
    { label:"HBM3e (B200)", bw:"~8 TB/s", cap:"192 GB", lat:"~100 ns", color:"#ff8030", w:90 },
    { label:"HBM3 (H100)", bw:"~3.4 TB/s", cap:"80 GB", lat:"~100 ns", color:"#e8a020", w:120 },
    { label:"GDDR7 (RTX 5090)", bw:"~1.8 TB/s", cap:"32 GB", lat:"~200 ns", color:"#d0c010", w:150 },
    { label:"GDDR6X (RTX 4090)", bw:"~1 TB/s", cap:"24 GB", lat:"~300 ns", color:"#a0c820", w:180 },
    { label:"Unified LPDDR5X (M4 Ultra)", bw:"~819 GB/s", cap:"256 GB", lat:"~80 ns", color:"#50c860", w:210 },
    { label:"DDR5 8-Ch Server", bw:"~310 GB/s", cap:"1.5 TB", lat:"~60 ns", color:"#30a0d0", w:240 },
    { label:"DDR5 Dual-Ch Desktop", bw:"~80 GB/s", cap:"128 GB", lat:"~60 ns", color:"#4070e0", w:270 },
    { label:"NVMe SSD (offload)", bw:"~7 GB/s", cap:"8+ TB", lat:"~10 µs", color:"#7050c0", w:300 },
  ];
  return (
    <div style={{ padding:20 }}>
      <h3 style={{ color:"#e8d5b7", fontSize:16, marginBottom:4, fontFamily:"'Playfair Display', serif" }}>Memory Hierarchy — Bandwidth vs Capacity Tradeoff</h3>
      <p style={{ color:"#8a8a9a", fontSize:11, marginBottom:20 }}>Each tier trades bandwidth for capacity. LLM inference lives in the bandwidth-bound regime.</p>
      <div style={{ display:"flex", flexDirection:"column", alignItems:"center", gap:4 }}>
        {tiers.map((t, i) => (
          <div key={i} style={{ display:"flex", alignItems:"center", gap:12, width:"100%" }}>
            <div style={{
              width: t.w, height:32, background:`linear-gradient(90deg, ${t.color}30, ${t.color}60)`,
              border:`1px solid ${t.color}80`, borderRadius:3,
              display:"flex", alignItems:"center", justifyContent:"center",
              fontSize:9, color:"#e0e0e8", fontWeight:600, marginLeft: (300 - t.w) / 2,
              letterSpacing:"0.02em"
            }}>{t.label}</div>
            <div style={{ fontSize:11, color:"#c8c8d0", fontFamily:"'JetBrains Mono', monospace", minWidth:100 }}>
              BW: {t.bw}
            </div>
            <div style={{ fontSize:11, color:"#8a8a9a", fontFamily:"'JetBrains Mono', monospace", minWidth:80 }}>
              Cap: {t.cap}
            </div>
            <div style={{ fontSize:10, color:"#6a6a7a", fontFamily:"'JetBrains Mono', monospace" }}>
              Lat: {t.lat}
            </div>
          </div>
        ))}
      </div>
      <div style={{ marginTop:16, display:"flex", justifyContent:"center", gap:30 }}>
        <div style={{ display:"flex", alignItems:"center", gap:6 }}>
          <div style={{ width:8, height:8, background:"#ff4060", borderRadius:"50%" }} />
          <span style={{ fontSize:10, color:"#8a8a9a" }}>↑ Higher BW, Lower Capacity</span>
        </div>
        <div style={{ display:"flex", alignItems:"center", gap:6 }}>
          <div style={{ width:8, height:8, background:"#7050c0", borderRadius:"50%" }} />
          <span style={{ fontSize:10, color:"#8a8a9a" }}>↓ Lower BW, Higher Capacity</span>
        </div>
      </div>
    </div>
  );
}

function SizingMatrix({ selectedModel, selectedQuant }) {
  const model = MODELS.find(m => m.id === selectedModel);
  const quant = QUANTS.find(q => q.id === selectedQuant);

  const results = HARDWARE.map(hw => ({
    hw,
    ...estimatePerf(hw, model, quant)
  }));

  const maxTG = Math.max(...results.map(r => r.tgTps), 1);
  const maxPP = Math.max(...results.map(r => r.ppTps), 1);

  return (
    <div style={{ overflowX:"auto" }}>
      <table style={{ width:"100%", borderCollapse:"collapse", fontSize:11 }}>
        <thead>
          <tr style={{ borderBottom:"2px solid #2a2a3e" }}>
            {["Hardware Class","Chip / Config","Memory Type","BW (GB/s)","Capacity","SIMD / Accel","Model Fits?","Est. Gen (tok/s)","Est. Prefill (tok/s)"].map((h,i) => (
              <th key={i} style={{ padding:"10px 8px", textAlign:"left", color:"#e8d5b7", fontWeight:700, fontSize:10, letterSpacing:"0.05em", textTransform:"uppercase", whiteSpace:"nowrap" }}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {results.map((r, i) => {
            const fit = r.fits ? (r.hw.cls === "GPU" || r.hw.cls === "Multi-GPU") ? "✅ VRAM" : "✅ RAM" : r.activeFits && model.arch.startsWith("MoE") ? "⚠️ Active only" : "❌ Too large";
            const fitColor = r.fits ? "#30b050" : r.activeFits ? "#d0a030" : "#d03030";
            const dimmed = !r.fits && !r.activeFits;
            return (
              <tr key={i} style={{ borderBottom:"1px solid #1a1a2e", opacity: dimmed ? 0.35 : 1, transition:"opacity 0.3s" }}>
                <td style={{ padding:"8px", whiteSpace:"nowrap" }}>
                  <Badge color={
                    r.hw.cls === "CPU" ? "#3966c3" :
                    r.hw.cls === "APU" ? "#c04820" :
                    r.hw.cls === "GPU" ? "#30b050" :
                    "#7030a0"
                  }>{r.hw.cls}</Badge>
                  <span style={{ color:"#c8c8d0", marginLeft:6 }}>{r.hw.tier}</span>
                </td>
                <td style={{ padding:"8px", color:"#a0a0b0", fontFamily:"'JetBrains Mono', monospace", fontSize:10 }}>{r.hw.chip}</td>
                <td style={{ padding:"8px" }}>
                  <span style={{ color: r.hw.unified ? "#e8a020" : "#8a8a9a", fontSize:10 }}>
                    {r.hw.memType}{r.hw.unified ? " ★" : ""}{r.hw.coherent && r.hw.cls !== "GPU" ? " ◆" : ""}
                  </span>
                </td>
                <td style={{ padding:"8px", fontFamily:"'JetBrains Mono', monospace", color:"#e0e0e8", fontWeight:700 }}>
                  {r.hw.bw.toLocaleString()}
                </td>
                <td style={{ padding:"8px", fontFamily:"'JetBrains Mono', monospace", color:"#a0a0b0", fontSize:10 }}>
                  {r.hw.capacity >= 1000 ? `${(r.hw.capacity/1024).toFixed(1)} TB` : `${r.hw.capacity} GB`}
                </td>
                <td style={{ padding:"8px", color:"#8a8a9a", fontSize:10 }}>{r.hw.simd}</td>
                <td style={{ padding:"8px", color:fitColor, fontWeight:600, fontSize:10 }}>
                  {fit}
                  <div style={{ fontSize:9, color:"#6a6a7a" }}>need {r.modelSizeGB} GB</div>
                </td>
                <td style={{ padding:"8px" }}>
                  {(r.fits || r.activeFits) ? (
                    <BarChart value={r.tgTps} max={maxTG} color={r.hw.color} label={`${r.tgTps} t/s`} width={120} />
                  ) : <span style={{ color:"#4a4a5a", fontSize:10 }}>N/A</span>}
                </td>
                <td style={{ padding:"8px" }}>
                  {(r.fits || r.activeFits) ? (
                    <BarChart value={r.ppTps} max={maxPP} color={r.hw.color} label={`${r.ppTps} t/s`} width={120} />
                  ) : <span style={{ color:"#4a4a5a", fontSize:10 }}>N/A</span>}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <div style={{ marginTop:12, display:"flex", gap:20, fontSize:10, color:"#6a6a7a" }}>
        <span>★ Unified memory (CPU+GPU share pool)</span>
        <span>◆ Cache-coherent</span>
        <span>Estimates assume single-stream batch=1 inference</span>
      </div>
    </div>
  );
}

function BWvsCapScatter({ selectedModel, selectedQuant }) {
  const model = MODELS.find(m => m.id === selectedModel);
  const quant = QUANTS.find(q => q.id === selectedQuant);
  const modelSizeGB = quant.id === "Q4" ? model.sizeQ4 : quant.id === "Q8" ? (model.sizeQ4 * 2) : model.sizeFP16;

  const W = 700, H = 400, PAD = { t:40, r:30, b:60, l:70 };
  const plotW = W - PAD.l - PAD.r;
  const plotH = H - PAD.t - PAD.b;

  const maxBW = 40000;
  const maxCap = 1200;

  const logX = (v) => PAD.l + (Math.log10(Math.max(v, 1)) / Math.log10(maxBW)) * plotW;
  const logY = (v) => PAD.t + plotH - (Math.log10(Math.max(v, 1)) / Math.log10(maxCap)) * plotH;

  return (
    <div style={{ padding:20 }}>
      <h3 style={{ color:"#e8d5b7", fontSize:16, marginBottom:4, fontFamily:"'Playfair Display', serif" }}>Bandwidth vs Capacity — Hardware Landscape</h3>
      <p style={{ color:"#8a8a9a", fontSize:11, marginBottom:16 }}>
        Dashed line = model size ({modelSizeGB} GB for {model.name} @ {quant.label}). Hardware below the line cannot fit the model.
      </p>
      <svg width={W} height={H} style={{ background:"#0d0d1a", borderRadius:4 }}>
        {/* Grid */}
        {[10,100,1000,10000].map(v => (
          <g key={`gx${v}`}>
            <line x1={logX(v)} y1={PAD.t} x2={logX(v)} y2={PAD.t+plotH} stroke="#1a1a2e" strokeWidth={1} />
            <text x={logX(v)} y={PAD.t+plotH+16} fill="#6a6a7a" fontSize={9} textAnchor="middle" fontFamily="JetBrains Mono, monospace">{v >= 1000 ? `${v/1000}k` : v}</text>
          </g>
        ))}
        {[1,10,100,1000].map(v => (
          <g key={`gy${v}`}>
            <line x1={PAD.l} y1={logY(v)} x2={PAD.l+plotW} y2={logY(v)} stroke="#1a1a2e" strokeWidth={1} />
            <text x={PAD.l-8} y={logY(v)+3} fill="#6a6a7a" fontSize={9} textAnchor="end" fontFamily="JetBrains Mono, monospace">{v >= 1000 ? `${v/1000} TB` : `${v} GB`}</text>
          </g>
        ))}
        {/* Axis labels */}
        <text x={PAD.l + plotW/2} y={H - 8} fill="#8a8a9a" fontSize={11} textAnchor="middle" fontWeight={600}>Memory Bandwidth (GB/s) — log scale</text>
        <text x={14} y={PAD.t + plotH/2} fill="#8a8a9a" fontSize={11} textAnchor="middle" fontWeight={600} transform={`rotate(-90, 14, ${PAD.t+plotH/2})`}>Capacity (GB) — log</text>

        {/* Model size threshold line */}
        <line x1={PAD.l} y1={logY(modelSizeGB)} x2={PAD.l+plotW} y2={logY(modelSizeGB)} stroke="#d03030" strokeWidth={1.5} strokeDasharray="6,4" />
        <text x={PAD.l+plotW-4} y={logY(modelSizeGB)-6} fill="#d03030" fontSize={9} textAnchor="end" fontWeight={600}>{modelSizeGB} GB needed</text>

        {/* Hardware dots */}
        {HARDWARE.map((hw, i) => {
          const x = logX(hw.bw);
          const y = logY(hw.capacity);
          const fits = hw.capacity >= modelSizeGB;
          return (
            <g key={hw.id}>
              <circle cx={x} cy={y} r={fits ? 7 : 5} fill={hw.color} opacity={fits ? 0.9 : 0.3} stroke={fits ? "#fff" : "none"} strokeWidth={1} />
              <text x={x + 10} y={y + 3} fill={fits ? "#c8c8d0" : "#4a4a5a"} fontSize={8} fontFamily="JetBrains Mono, monospace">
                {hw.tier.length > 22 ? hw.tier.slice(0,20)+"…" : hw.tier}
              </text>
            </g>
          );
        })}

        {/* Legend */}
        {[{c:"#3966c3",l:"CPU"},{c:"#c04820",l:"APU/Unified"},{c:"#30b050",l:"GPU (GDDR/HBM)"},{c:"#7030a0",l:"Multi-GPU"}].map((leg, i) => (
          <g key={leg.l}>
            <circle cx={PAD.l + i * 130 + 10} cy={PAD.t + 12} r={5} fill={leg.c} />
            <text x={PAD.l + i * 130 + 20} y={PAD.t + 15} fill="#a0a0b0" fontSize={9}>{leg.l}</text>
          </g>
        ))}
      </svg>
    </div>
  );
}

function ModelArchCompare() {
  return (
    <div style={{ padding:20 }}>
      <h3 style={{ color:"#e8d5b7", fontSize:16, marginBottom:4, fontFamily:"'Playfair Display', serif" }}>Model Architecture — Total vs Active Parameters</h3>
      <p style={{ color:"#8a8a9a", fontSize:11, marginBottom:20 }}>MoE models activate only a fraction of parameters per token, dramatically reducing bandwidth needs per token.</p>
      <div style={{ display:"grid", gridTemplateColumns:"repeat(4, 1fr)", gap:16 }}>
        {MODELS.map(m => {
          const activeRatio = m.active / m.params;
          return (
            <div key={m.id} style={{ background:"#12122a", borderRadius:6, padding:16, border:"1px solid #1a1a2e" }}>
              <div style={{ fontSize:13, color:"#e8d5b7", fontWeight:700, marginBottom:4 }}>{m.name}</div>
              <div style={{ fontSize:10, color:"#8a8a9a", marginBottom:12 }}>{m.arch}</div>
              {/* Total params bar */}
              <div style={{ marginBottom:8 }}>
                <div style={{ fontSize:9, color:"#6a6a7a", marginBottom:2 }}>Total: {m.params}B params</div>
                <div style={{ width:"100%", height:20, background:"#1a1a2e", borderRadius:2, position:"relative", overflow:"hidden" }}>
                  <div style={{ width:"100%", height:"100%", background:"#3966c355", borderRadius:2 }} />
                  <div style={{
                    position:"absolute", top:0, left:0,
                    width:`${activeRatio * 100}%`, height:"100%",
                    background: activeRatio < 1 ? "linear-gradient(90deg, #e8a020, #ff6030)" : "linear-gradient(90deg, #3966c3, #5b8def)",
                    borderRadius:2
                  }} />
                </div>
              </div>
              <div style={{ fontSize:9, color:"#6a6a7a" }}>Active: {m.active}B ({(activeRatio * 100).toFixed(1)}%)</div>
              <div style={{ marginTop:10, display:"grid", gridTemplateColumns:"1fr 1fr", gap:4, fontSize:10 }}>
                <div><span style={{ color:"#6a6a7a" }}>Q4: </span><span style={{ color:"#c8c8d0", fontFamily:"'JetBrains Mono', monospace" }}>{m.sizeQ4} GB</span></div>
                <div><span style={{ color:"#6a6a7a" }}>FP16: </span><span style={{ color:"#c8c8d0", fontFamily:"'JetBrains Mono', monospace" }}>{m.sizeFP16} GB</span></div>
              </div>
              {m.moeExperts > 0 && (
                <div style={{ marginTop:8, fontSize:9, color:"#e8a020", background:"#e8a02010", padding:"4px 6px", borderRadius:3 }}>
                  {m.moeExperts} experts, {m.moeActive} active/token → {Math.round(m.sizeQ4 * m.active / m.params * 1.15)} GB effective BW/token
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function FutureGPURoadmap() {
  const gens = [
    { gen:"Ada (2022)", chip:"RTX 4090", mem:"GDDR6X", vram:24, bw:1008, color:"#30b050" },
    { gen:"Blackwell Consumer (2025)", chip:"RTX 5090", mem:"GDDR7", vram:32, bw:1792, color:"#20a040" },
    { gen:"Blackwell DC (2025)", chip:"B200", mem:"HBM3e", vram:192, bw:8000, color:"#005010" },
    { gen:"Rubin DC (2026–27)", chip:"R100 (proj.)", mem:"HBM4", vram:288, bw:12000, color:"#004040" },
    { gen:"Consumer (2027+)", chip:"RTX 6090 (proj.)", mem:"GDDR7+", vram:48, bw:2400, color:"#206838" },
  ];
  const maxBW = 12000;
  return (
    <div style={{ padding:20 }}>
      <h3 style={{ color:"#e8d5b7", fontSize:16, marginBottom:4, fontFamily:"'Playfair Display', serif" }}>GPU Roadmap — VRAM Capacity & Bandwidth Scaling</h3>
      <p style={{ color:"#8a8a9a", fontSize:11, marginBottom:16 }}>Future GPUs will close the capacity gap. HBM4 and GDDR7+ bring both more bandwidth and more capacity per chip.</p>
      <div style={{ display:"flex", flexDirection:"column", gap:10 }}>
        {gens.map((g, i) => (
          <div key={i} style={{ display:"grid", gridTemplateColumns:"200px 80px 120px 1fr", alignItems:"center", gap:12 }}>
            <div>
              <div style={{ fontSize:12, color:"#e0e0e8", fontWeight:600 }}>{g.gen}</div>
              <div style={{ fontSize:10, color:"#6a6a7a" }}>{g.chip} — {g.mem}</div>
            </div>
            <div style={{ fontFamily:"'JetBrains Mono', monospace", fontSize:11, color:"#e8d5b7" }}>
              {g.vram} GB
            </div>
            <div style={{ fontFamily:"'JetBrains Mono', monospace", fontSize:11, color:"#c8c8d0" }}>
              {g.bw.toLocaleString()} GB/s
            </div>
            <div style={{ display:"flex", alignItems:"center", gap:6 }}>
              <div style={{ width:"100%", maxWidth:300, height:18, background:"#1a1a2e", borderRadius:2, overflow:"hidden" }}>
                <div style={{
                  width:`${(g.bw / maxBW) * 100}%`, height:"100%",
                  background:`linear-gradient(90deg, ${g.color}88, ${g.color})`,
                  borderRadius:2
                }} />
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Main App ──────────────────────────────────────────────────────────────────

export default function App() {
  const [tab, setTab] = useState("sizing");
  const [selectedModel, setSelectedModel] = useState("llama-70b");
  const [selectedQuant, setSelectedQuant] = useState("Q4");

  return (
    <div style={{
      minHeight:"100vh", background:"#0a0a18",
      fontFamily:"'DM Sans', 'Segoe UI', sans-serif", color:"#c8c8d0",
      display:"flex", flexDirection:"column"
    }}>
      <link href="https://fonts.googleapis.com/css2?family=Playfair+Display:wght@600;700&family=DM+Sans:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet" />

      {/* Header */}
      <header style={{ padding:"24px 32px 0", borderBottom:"1px solid #1a1a2e" }}>
        <div style={{ display:"flex", alignItems:"baseline", gap:12, marginBottom:8 }}>
          <h1 style={{ fontSize:22, fontFamily:"'Playfair Display', serif", color:"#e8d5b7", margin:0, fontWeight:700 }}>
            LLM Inference Sizing Model
          </h1>
          <span style={{ fontSize:11, color:"#6a6a7a" }}>CPU · APU · GPU · Memory Hierarchy</span>
        </div>
        <p style={{ fontSize:12, color:"#8a8a9a", margin:"0 0 16px", maxWidth:800 }}>
          Interactive comparison of hardware platforms for large model inference. Performance estimates based on memory bandwidth constraints, SIMD capabilities, and model architecture (dense vs MoE).
        </p>

        {/* Model + Quant selectors */}
        <div style={{ display:"flex", gap:24, marginBottom:16, flexWrap:"wrap" }}>
          <div>
            <label style={{ fontSize:10, color:"#6a6a7a", textTransform:"uppercase", letterSpacing:"0.05em", display:"block", marginBottom:4 }}>Model</label>
            <div style={{ display:"flex", gap:6 }}>
              {MODELS.map(m => (
                <button key={m.id} onClick={() => setSelectedModel(m.id)} style={{
                  padding:"6px 14px", border: selectedModel === m.id ? "1px solid #e8d5b7" : "1px solid #2a2a3e",
                  borderRadius:4, background: selectedModel === m.id ? "#1a1a2e" : "transparent",
                  color: selectedModel === m.id ? "#e8d5b7" : "#6a6a7a",
                  fontSize:11, cursor:"pointer", fontWeight:600, transition:"all 0.15s"
                }}>
                  {m.name.replace("DeepSeek R1 671B","DS-R1 671B").replace("Llama 3.3 ","").replace("Llama 3.1 ","").replace("GPT-OSS ","GPT-OSS ")}
                </button>
              ))}
            </div>
          </div>
          <div>
            <label style={{ fontSize:10, color:"#6a6a7a", textTransform:"uppercase", letterSpacing:"0.05em", display:"block", marginBottom:4 }}>Quantization</label>
            <div style={{ display:"flex", gap:6 }}>
              {QUANTS.map(q => (
                <button key={q.id} onClick={() => setSelectedQuant(q.id)} style={{
                  padding:"6px 14px", border: selectedQuant === q.id ? "1px solid #e8d5b7" : "1px solid #2a2a3e",
                  borderRadius:4, background: selectedQuant === q.id ? "#1a1a2e" : "transparent",
                  color: selectedQuant === q.id ? "#e8d5b7" : "#6a6a7a",
                  fontSize:11, cursor:"pointer", fontWeight:600, transition:"all 0.15s"
                }}>
                  {q.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div style={{ display:"flex", gap:0 }}>
          <Tab active={tab==="sizing"} onClick={() => setTab("sizing")}>Sizing Matrix</Tab>
          <Tab active={tab==="scatter"} onClick={() => setTab("scatter")}>BW vs Capacity</Tab>
          <Tab active={tab==="memory"} onClick={() => setTab("memory")}>Memory Hierarchy</Tab>
          <Tab active={tab==="arch"} onClick={() => setTab("arch")}>Model Architectures</Tab>
          <Tab active={tab==="roadmap"} onClick={() => setTab("roadmap")}>GPU Roadmap</Tab>
        </div>
      </header>

      {/* Content */}
      <main style={{ flex:1, padding:"0 32px 32px", overflowX:"auto" }}>
        {tab === "sizing" && <SizingMatrix selectedModel={selectedModel} selectedQuant={selectedQuant} />}
        {tab === "scatter" && <BWvsCapScatter selectedModel={selectedModel} selectedQuant={selectedQuant} />}
        {tab === "memory" && <MemoryHierarchyDiagram />}
        {tab === "arch" && <ModelArchCompare />}
        {tab === "roadmap" && <FutureGPURoadmap />}
      </main>

      {/* Footer */}
      <footer style={{ padding:"12px 32px", borderTop:"1px solid #1a1a2e", fontSize:10, color:"#4a4a5a", textAlign:"center" }}>
        Estimates based on theoretical BW ÷ model_size with efficiency factors (CPU ~65%, APU ~72%, GPU ~85%, Multi-GPU ~78%). 
        Real performance varies with NUMA topology, quantization kernels, KV-cache size, batch size, and software stack (llama.cpp, vLLM, kTransformers).
        Single-stream batch=1 generation. Prefill estimates are rougher and depend heavily on compute/SIMD utilization.
      </footer>
    </div>
  );
}
