import { Nav } from "./components/Nav";
import { Hero } from "./components/Hero";
import { Values } from "./components/Values";
import { HowItWorks } from "./components/HowItWorks";
import { Features } from "./components/Features";
import { UseCases } from "./components/UseCases";
import { CLIShowcase } from "./components/CLIShowcase";
import { FinalCTA } from "./components/FinalCTA";
import { Footer } from "./components/Footer";

export default function App() {
  return (
    <div
      style={{
        background: "#0d1117",
        minHeight: "100vh",
        fontFamily: "'Inter', 'Noto Sans SC', system-ui, sans-serif",
        WebkitFontSmoothing: "antialiased",
        MozOsxFontSmoothing: "grayscale",
      }}
    >
      <Nav />
      <main>
        <Hero />
        <Values />
        <Features />
        <HowItWorks />
        <UseCases />
        <CLIShowcase />
        <FinalCTA />
      </main>
      <Footer />
    </div>
  );
}
