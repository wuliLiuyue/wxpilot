import { FadeUp, StaggerContainer, StaggerItem } from "./animations";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";

const cases = [
  "AI Agent 驱动的小程序任务执行",
  "测试与回归过程辅助",
  "演示流程自动推进",
  "重复操作批量处理",
  "团队调试与问题复现",
];

export function UseCases() {
  return (
    <section
      id="use-cases"
      style={{
        background: "#0f161e",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "96px 0",
        fontFamily: SANS,
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        <div style={{ display: "grid", gridTemplateColumns: "240px 1fr", gap: 64, alignItems: "start" }} className="use-cases-grid">
          <FadeUp>
            <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 16, fontFamily: MONO }}>
              Use cases
            </div>
            <h2 style={{ fontSize: "clamp(20px, 2.5vw, 28px)", color: "#e8e6e1", fontWeight: 500, lineHeight: 1.3, letterSpacing: "-0.02em", margin: 0 }}>
              适合这些场景
            </h2>
          </FadeUp>

          <StaggerContainer style={{ display: "flex", flexDirection: "column" }}>
            {cases.map((c, i) => (
              <StaggerItem
                key={c}
                style={{
                  display: "flex",
                  alignItems: "baseline",
                  gap: 24,
                  padding: "24px 0",
                  borderBottom: i < cases.length - 1 ? "1px solid rgba(50,68,90,0.35)" : "none",
                  cursor: "default",
                  transition: "all 0.2s",
                }}
                className="use-case-row"
              >
                <span style={{ fontSize: 11, color: "#2a3d4f", fontFamily: MONO, minWidth: 24, flexShrink: 0 }}>
                  {String(i + 1).padStart(2, "0")}
                </span>
                <span
                  style={{
                    fontSize: "clamp(18px, 2.5vw, 26px)",
                    color: "#c8d4e0",
                    fontWeight: 400,
                    lineHeight: 1.3,
                    letterSpacing: "-0.01em",
                  }}
                >
                  {c}
                </span>
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 16 16"
                  fill="none"
                  style={{ marginLeft: "auto", flexShrink: 0, opacity: 0.3 }}
                  className="use-case-arrow"
                >
                  <path d="M3 8h10M9 4l4 4-4 4" stroke="#7ab89a" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </StaggerItem>
            ))}
          </StaggerContainer>
        </div>
      </div>

      <style>{`
        .use-case-row:hover .use-case-arrow { opacity: 0.7 !important; }
        .use-case-row:hover > span:last-of-type { color: #e8e6e1 !important; }
        .use-cases-grid { grid-template-columns: 240px 1fr; }
        @media (max-width: 768px) {
          .use-cases-grid { grid-template-columns: 1fr !important; gap: 40px !important; }
        }
      `}</style>
    </section>
  );
}
