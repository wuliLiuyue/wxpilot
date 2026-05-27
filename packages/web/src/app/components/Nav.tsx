import { useState, useEffect } from "react";
import { Menu, X } from "lucide-react";

const links = [
  ["功能", "features"],
  ["使用方式", "how-it-works"],
  ["场景", "use-cases"],
  ["CLI", "cli"],
];

function scrollToSection(id: string) {
  const el = document.getElementById(id);
  if (!el) return;
  const top = el.getBoundingClientRect().top + window.scrollY - 72;
  window.scrollTo({ top, behavior: "smooth" });
}

export function Nav() {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState("");

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) setActive(entry.target.id);
        });
      },
      { threshold: 0.25, rootMargin: "-64px 0px -45% 0px" }
    );
    links.forEach(([, id]) => {
      const el = document.getElementById(id);
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, []);

  return (
    <nav
      className="fixed top-0 left-0 right-0 z-50"
      style={{
        background: "rgba(13, 17, 23, 0.90)",
        backdropFilter: "blur(20px)",
        borderBottom: "1px solid rgba(50, 68, 90, 0.45)",
        fontFamily: "'Inter', 'Noto Sans SC', sans-serif",
      }}
    >
      <div className="max-w-6xl mx-auto px-6 h-14 flex items-center justify-between">
        <a
          href="#"
          onClick={(e) => { e.preventDefault(); window.scrollTo({ top: 0, behavior: "smooth" }); }}
          className="flex items-center gap-1.5"
          style={{ textDecoration: "none" }}
        >
          <span style={{ fontFamily: "'JetBrains Mono', monospace", color: "#e8e6e1", letterSpacing: "-0.02em", fontSize: 15 }}>
            wxpilot
          </span>
          <span style={{ color: "#4d8a6a", fontFamily: "'JetBrains Mono', monospace", fontSize: 15, animation: "blink 1.2s step-end infinite" }}>
            _
          </span>
        </a>

        <div className="hidden md:flex items-center gap-7">
          {links.map(([label, id]) => {
            const isActive = active === id;
            return (
              <a
                key={id}
                href={`#${id}`}
                onClick={(e) => { e.preventDefault(); scrollToSection(id); }}
                style={{
                  fontSize: 14,
                  textDecoration: "none",
                  color: isActive ? "#e8e6e1" : "#7a8ba8",
                  transition: "color 0.2s",
                }}
              >
                {label}
              </a>
            );
          })}
        </div>

        <button
          className="md:hidden p-2"
          style={{ color: "#7a8ba8", background: "none", border: "none", cursor: "pointer" }}
          onClick={() => setOpen(!open)}
        >
          {open ? <X size={18} /> : <Menu size={18} />}
        </button>
      </div>

      {open && (
        <div
          style={{ background: "#0d1117", borderBottom: "1px solid rgba(50, 68, 90, 0.45)" }}
          className="md:hidden px-6 py-5 flex flex-col gap-4"
        >
          {links.map(([label, id]) => (
            <a
              key={id}
              href={`#${id}`}
              style={{ color: active === id ? "#e8e6e1" : "#7a8ba8", fontSize: 14, textDecoration: "none" }}
              onClick={(e) => { e.preventDefault(); scrollToSection(id); setOpen(false); }}
            >
              {label}
            </a>
          ))}
        </div>
      )}

      <style>{`@keyframes blink { 0%,50%{opacity:1}51%,100%{opacity:0} }`}</style>
    </nav>
  );
}
