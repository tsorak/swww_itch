import { Transition } from "solid-transition-group";

export default function T({ s, type, ...props }) {
  type ??= "fade-up";

  const transition = (() => {
    if (typeof type == "object") {
      return type;
    }

    const fadeUp = {
      in: [
        {
          opacity: 0,
          ["transform-origin"]: "top",
          transform: `translateY(-10%) scale(0.9)`,
        },
        {
          opacity: 1,
          transform: "",
        },
      ],
      out: [
        {
          opacity: 1,
          transform: "",
        },
        {
          opacity: 0,
          ["transform-origin"]: "top",
          transform: `translateY(-10%) scale(0.9)`,
        },
      ],
    };

    const fade = {
      in: [{ opacity: 0 }, { opacity: 1 }],
      out: [{ opacity: 1 }, { opacity: 0 }],
    };

    if (type == "fade-up") {
      return fadeUp;
    } else {
      return fade;
    }
  })();

  transition.in.map((v) => {
    v["pointer-events"] = "none";
    return v;
  });
  transition.out.map((v) => {
    v["pointer-events"] = "none";
    return v;
  });

  const direction = () => {
    const SENS = "20";

    const r = Math.random();
    if (r > 0.0 && r < 0.25) return `translateX(-${SENS}%)`;
    else if (r > 0.25 && r < 0.5) return `translateY(-${SENS}%)`;
    else if (r > 0.5 && r < 0.75) return `translateY(${SENS}%)`;
    else return `translateX(${SENS}%)`;

    // { opacity: 0, transform: `${direction()} scale(0.9)` },
    // { opacity: 0, transform: `${direction()} scale(0.9)` },
  };

  return (
    <Transition
      onEnter={(el, done) => {
        const anim = el.animate(transition.in, {
          easing: "ease-in-out",
          duration: 250,
        });

        anim.finished.then(() => {
          s?.isRouting.set(false);
          done();
        });
      }}
      onExit={(el, done) => {
        s?.isRouting.set(true);

        const anim = el.animate(transition.out, {
          easing: "ease-in-out",
          duration: 250,
        });

        anim.finished.then(() => {
          s?.busyExiting.set("");
          done();
        });
      }}
    >
      {props.children}
    </Transition>
  );
}
