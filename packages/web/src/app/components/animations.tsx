import { motion, type HTMLMotionProps } from "motion/react";

const EASE = [0.25, 0.46, 0.45, 0.94] as const;

interface FadeUpProps extends HTMLMotionProps<"div"> {
  delay?: number;
}

export function FadeUp({ children, delay = 0, style, ...props }: FadeUpProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 22 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-72px" }}
      transition={{ duration: 0.55, ease: EASE, delay }}
      style={style}
      {...props}
    >
      {children}
    </motion.div>
  );
}

export function StaggerContainer({ children, style, className, ...props }: HTMLMotionProps<"div">) {
  return (
    <motion.div
      initial="hidden"
      whileInView="show"
      viewport={{ once: true, margin: "-72px" }}
      variants={{ hidden: {}, show: { transition: { staggerChildren: 0.09 } } }}
      style={style}
      className={className}
      {...props}
    >
      {children}
    </motion.div>
  );
}

export function StaggerItem({ children, style, className, ...props }: HTMLMotionProps<"div">) {
  return (
    <motion.div
      variants={{
        hidden: { opacity: 0, y: 20 },
        show: { opacity: 1, y: 0, transition: { duration: 0.5, ease: EASE } },
      }}
      style={style}
      className={className}
      {...props}
    >
      {children}
    </motion.div>
  );
}
