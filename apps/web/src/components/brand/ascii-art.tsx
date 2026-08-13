import { useEffect, useRef } from "react";
import { HAND_DOWN, HAND_UP } from "@/components/brand/hand-maps";

const STATUE = `++=========+=====+++++++++++******+++++++++++++++++++++*****#############%%######***#######*#%#######****#####
..=+=====+=======++++++++++++++*****+++++++++++++++++*++****#############%####%%%@@@%%#*%#########***######***
+===++++====++++++++++++++++++++********+++++++++***++*+*****###########%%%%%@@@@@@@@@@%#*####*##########*#**#
***+++++++==+++++++++++++++**++++**#**##*+*+++++++*+++********#*#####%%%%@@@@@@@@@@@%%%%%%###*#%*=+#*****++###
*****+++++++==++++++++**************#####*****************#*****##%%%%@%@@@@@%%%%%##%###%%##*++++*****######*#
*********++++++++++++*************#######******%%###*******########%%%%%%@@%%%######%%#*+++++******###%%%%%##*
**********++++++++***********#***###%@%*+#%###*%*:.-%##*##*#########%%%%%%###%%%%%%%#+++++++++****##%%%%%%####
**********+++++***********###***###%@@@#**@@@*=+:  -**#@@@%#######%##########%%%%#%*++***++++++++*#%%%########
****####***********###**##***###@+--*@#==#%#%@@%#:::=+#*++-:=+=###########%%%%%%#=*******+++++++++##%#########
#**####***********#########%##%%@==::=.   :--=*@%%%%@@@@%#- .:+%%%######%%##******+****+**+++*******#######%##
*####****#*********#####**+#%@@@@#*++=-=-- .-:=#%@@@@@@@@@#=.  .:-######****++*********************+**########
-+##+*###*#********####*%*=#@@#=++*%@%%##**%%##@@@@@@@@@@@@@%= .. =*#######*+******************#******##**####
+===-=**###**#****##%%%#*..-*#-  :------*+-=*%@@@%*+==+***#@@@* .:.+*######**+++++******#***#####********###**
+++++++++*#######%#***##++**#*-.     ..:-+=*##*-::::.     .=*%%*:-+***###****++++++++*****#*######****++******
+++++**+=+##*#####+.=+*+==-:::+*- .::=. :+%%*-.  .-=:     .#%=.==+#**#####*****++++++*+*****#####*************
++********###%#*#+- -..:-   .=*::+==+-.-*@@@%%##**+-=- .:=*#%@+  ::=+#######***+++++++*****++********#####****
******###########= =.      ...-=+*=++-=#%%#%@@@@@@@%%%#%@@@@%@@*::  .=######***++********++++*****=+++#+*%####
****++#%#########*--==+-::.    .:::   :=--+++***#%@@@@@@###*=--::.. :+*#####************++++*+++**++++=-=**+#%
#++####%%%####*#+=+=-==-::..        .:...:=--:--+#%@@%%*#-       .-=+++**####***********++++***++++++++**++==*
##*#####%%###*#+---.:                 :+**+=--=+*##%%@@@@%*+-  :::=+*+++***#**#######**********+++++*****+****
*###*#########*=.       .. ...      .:=++***+*****#%@%+-==-:   +#..--++++++++*##########******+***************
***###########+.      ..=::... .:--::=++####***++++**+++==+=-: :+- .=+++***+++****##########****************##
#***#####*#####*:     ..       ..:. :--+**+++++++==--=##+:.  :.-+.:-+++*****++****###%%%#%#*****#######*#####*
#**#*###########-                    :=+=--==++++=++*%%@@@@#+-.-=-==+++++*******#######%%%******#############%
###**#######*****-..                 .:.  ..:..::------=----+%*+=+++=+++*****#***####%%%%%@%%%####%%%%###%%@@@`;

export function HeroAscii() {
  return (
    <pre
      aria-hidden="true"
      className="hero-ascii pointer-events-none hidden select-none justify-self-end lg:block"
    >
      {STATUE}
    </pre>
  );
}

export function EdgeDither({ side }: { side: "left" | "right" }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const wrap = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const cv = ref.current;
    const el = wrap.current;
    if (!cv || !el) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const CW = 12;
    const CH = 14;
    const RAMP = [" ", " ", " ", ".", ":", ">", "~", "×", "*", "#"];
    const right = side === "right";
    const ox = right ? 397 : 0;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);

    const resize = () => {
      const w = el.clientWidth;
      const h = el.clientHeight;
      cv.width = w * dpr;
      cv.height = h * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.font = '10px "Geist Mono", ui-monospace, monospace';
      ctx.textBaseline = "top";
    };

    const field = (x: number, y: number, t: number) => {
      const a = x * 0.55;
      const b = y * 0.35;
      const v =
        Math.sin(a + 2.1 * Math.sin(b * 0.9 + t) + t * 0.7) *
        Math.cos(b - 1.7 * Math.sin(a * 0.6 - t * 0.8));
      return 0.5 + 0.5 * v;
    };

    const draw = (t: number) => {
      const w = el.clientWidth;
      const h = el.clientHeight;
      if (!w || !h) return;
      const cols = Math.ceil(w / CW);
      const rows = Math.ceil(h / CH);
      ctx.clearRect(0, 0, w, h);
      for (let y = 0; y < rows; y += 1) {
        for (let x = 0; x < cols; x += 1) {
          const edge = right ? cols - 1 - x : x;
          const falloff = 1 - edge / cols;
          const v = field(x + ox, y, t) * falloff * 1.6;
          const g = RAMP[Math.min(RAMP.length - 1, (v * RAMP.length) | 0)];
          if (g === " ") continue;
          ctx.fillStyle = `rgba(139, 92, 246, ${(0.16 + v * 0.5).toFixed(2)})`;
          ctx.fillText(g, x * CW + 2, y * CH + 2);
        }
      }
    };

    resize();
    let last = 0;
    let raf = 0;
    let prev = 0;
    const ro = new ResizeObserver(() => {
      resize();
      draw(last);
    });
    ro.observe(el);

    if (reduced) {
      draw(0);
    } else {
      const tick = (ms: number) => {
        if (ms - prev > 66) {
          prev = ms;
          last = ms * 0.00022;
          draw(last);
        }
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    }

    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  }, [side]);

  return (
    <div
      ref={wrap}
      aria-hidden="true"
      className={`pointer-events-none absolute top-0 bottom-0 z-0 hidden w-[140px] [mask-image:linear-gradient(to_bottom,transparent,black_20%,black_80%,transparent)] xl:block ${
        side === "left" ? "left-0" : "right-0"
      }`}
    >
      <canvas ref={ref} className="block size-full" />
    </div>
  );
}

export function HandAscii({ variant }: { variant: "down" | "up" }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    const CW = 12;
    const CH = 14;
    const RAMP = [" ", " ", " ", ".", ":", ">", "~", "×", "*", "#"];
    const data = variant === "down" ? HAND_DOWN : HAND_UP;
    const ox = variant === "down" ? 101 : 811;
    const grid = data.split("\n");
    const cols = grid[0]?.length ?? 0;
    const rows = grid.length;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    cv.width = cols * CW * dpr;
    cv.height = rows * CH * dpr;
    cv.style.width = `${cols * CW}px`;
    cv.style.height = `${rows * CH}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.font = '10px "Geist Mono", ui-monospace, monospace';
    ctx.textBaseline = "top";

    const field = (x: number, y: number, t: number) => {
      const a = x * 0.55;
      const b = y * 0.35;
      const v =
        Math.sin(a + 2.1 * Math.sin(b * 0.9 + t) + t * 0.7) *
        Math.cos(b - 1.7 * Math.sin(a * 0.6 - t * 0.8));
      return 0.5 + 0.5 * v;
    };

    const draw = (t: number) => {
      ctx.clearRect(0, 0, cols * CW, rows * CH);
      for (let y = 0; y < rows; y += 1) {
        for (let x = 0; x < cols; x += 1) {
          const base = Number(grid[y]?.[x] ?? "0") / 9;
          if (!base) continue;
          const v = base * (0.72 + 0.58 * field(x + ox, y, t));
          const g = RAMP[Math.min(RAMP.length - 1, (v * RAMP.length) | 0)];
          if (g === " ") continue;
          ctx.fillStyle = `rgba(139, 92, 246, ${(0.16 + v * 0.5).toFixed(2)})`;
          ctx.fillText(g, x * CW + 2, y * CH + 2);
        }
      }
    };

    let raf = 0;
    if (reduced) {
      draw(0);
    } else {
      let prev = 0;
      const tick = (ms: number) => {
        if (ms - prev > 66) {
          prev = ms;
          draw(ms * 0.00022);
        }
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    }

    return () => cancelAnimationFrame(raf);
  }, [variant]);

  return (
    <canvas
      ref={ref}
      aria-hidden="true"
      className={`pointer-events-none absolute z-0 hidden [mask-image:linear-gradient(to_bottom,transparent,black_20%,black_80%,transparent)] lg:block ${
        variant === "down" ? "-top-80 left-0" : "bottom-0 right-0"
      }`}
    />
  );
}
