const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";

export function Footer() {
  return (
    <footer
      style={{
        background: "#0a0f16",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "48px 0",
        fontFamily: SANS,
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        <div
          style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: 24 }}
          className="footer-inner"
        >
          {/* Left: brand */}
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ fontFamily: MONO, color: "#e8e6e1", fontSize: 14 }}>wxpilot</span>
              <span style={{ fontFamily: MONO, color: "#4d8a6a", fontSize: 14 }}>_</span>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
              <span style={{ fontSize: 12, color: "#3a4a5a" }}>Built for developers and AI workflows</span>
              <span style={{ fontSize: 12, color: "#3a4a5a" }}>Focused on usability, control, and clarity</span>
            </div>
          </div>

          {/* Right: links */}
          <div style={{ display: "flex", gap: 28 }}>
            {[["GitHub", "#"]].map(([label, href]) => (
              <a
                key={label}
                href={href}
                style={{ fontSize: 13, color: "#4a5a6e", textDecoration: "none", fontFamily: SANS }}
                className="hover:text-[#7a8ba8] transition-colors duration-200"
              >
                {label}
              </a>
            ))}
          </div>
        </div>
      </div>

      <style>{`
        @media (max-width: 560px) {
          .footer-inner { flex-direction: column !important; align-items: flex-start !important; }
        }
      `}</style>
    </footer>
  );
}
