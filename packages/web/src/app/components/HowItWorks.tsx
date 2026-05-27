import { FadeUp, StaggerContainer, StaggerItem } from "./animations";

const SANS = "'Inter', 'Noto Sans SC', sans-serif";
const MONO = "'JetBrains Mono', monospace";

const steps = [
  { num: "1", label: "准备项目", body: "让工具识别当前工作目标，进入可操作状态。" },
  { num: "2", label: "连接会话", body: "建立当前操作上下文，准备执行页面任务。" },
  { num: "3", label: "查看与操作", body: "浏览当前页面，执行常见交互，推进任务流程。" },
  { num: "4", label: "记录与辅助调试", body: "保留关键结果与画面，帮助排查、验证和协作。" },
];

export function HowItWorks() {
  return (
    <section
      id="how-it-works"
      style={{
        background: "#0f161e",
        borderTop: "1px solid rgba(50,68,90,0.4)",
        padding: "96px 0",
        fontFamily: SANS,
      }}
    >
      <div className="max-w-6xl mx-auto px-6">
        <FadeUp style={{ marginBottom: 72, maxWidth: 480 }}>
          <div style={{ fontSize: 11, letterSpacing: "0.12em", color: "#4d8a6a", textTransform: "uppercase", marginBottom: 16, fontFamily: MONO }}>
            How it works
          </div>
          <h2 style={{ fontSize: "clamp(24px, 3vw, 36px)", color: "#e8e6e1", fontWeight: 500, lineHeight: 1.25, letterSpacing: "-0.02em", margin: "0 0 14px 0" }}>
            几步开始使用
          </h2>
          <p style={{ fontSize: 16, color: "#7a8ba8", lineHeight: 1.7, margin: 0 }}>
            用更低的心智负担，把自动化带入你的小程序工作流。
          </p>
        </FadeUp>

        {/* Steps */}
        <StaggerContainer style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 0, position: "relative" }} className="steps-grid">
          {/* Connecting line */}
          <div
            style={{
              position: "absolute",
              top: 20,
              left: "calc(12.5% + 0px)",
              right: "calc(12.5% + 0px)",
              height: 1,
              background: "linear-gradient(to right, rgba(50,68,90,0) 0%, rgba(50,68,90,0.6) 10%, rgba(50,68,90,0.6) 90%, rgba(50,68,90,0) 100%)",
            }}
            className="connecting-line"
          />

          {steps.map((step, i) => (
            <StaggerItem
              key={step.num}
              style={{
                padding: "0 24px 0 0",
                display: "flex",
                flexDirection: "column",
                gap: 20,
                position: "relative",
              }}
            >
              {/* Step number circle */}
              <div style={{ position: "relative", zIndex: 1 }}>
                <div
                  style={{
                    width: 40,
                    height: 40,
                    borderRadius: "50%",
                    border: "1px solid rgba(77,138,106,0.35)",
                    background: "#0f161e",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    boxShadow: "0 0 0 4px #0f161e",
                  }}
                >
                  <span style={{ fontSize: 13, color: "#4d8a6a", fontFamily: MONO }}>{step.num}</span>
                </div>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                <h3 style={{ fontSize: 16, color: "#e8e6e1", fontWeight: 500, lineHeight: 1.35, margin: 0, letterSpacing: "-0.01em" }}>
                  {step.label}
                </h3>
                <p style={{ fontSize: 14, color: "#7a8ba8", lineHeight: 1.7, margin: 0 }}>
                  {step.body}
                </p>
              </div>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>

      <style>{`
        .steps-grid { grid-template-columns: repeat(4, 1fr) !important; }
        @media (max-width: 768px) {
          .steps-grid { grid-template-columns: repeat(2, 1fr) !important; gap: 40px !important; }
          .connecting-line { display: none !important; }
        }
        @media (max-width: 480px) {
          .steps-grid { grid-template-columns: 1fr !important; }
        }
      `}</style>
    </section>
  );
}
