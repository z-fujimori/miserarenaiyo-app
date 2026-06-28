import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type OverlayItemType = "type1" | "type2";

const overlayLabels: Record<OverlayItemType, string> = {
  type1: "Type1",
  type2: "Type2",
};

function App() {
  const [itemType, setItemType] = useState<OverlayItemType>("type1");

  const startDragging = () => {
    void getCurrentWindow().startDragging();
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void invoke("hide_overlay");
      }
    };

    window.addEventListener("keydown", onKeyDown);

    void invoke<OverlayItemType>("get_overlay_item_type").then((value) => {
      setItemType(value);
    });

    let unlisten: (() => void) | undefined;
    void listen<OverlayItemType>("overlay-item-type-changed", (event) => {
      setItemType(event.payload);
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      unlisten?.();
    };
  }, []);

  return (
    <main
      className="overlay-root"
      data-item-type={itemType}
      onMouseDown={(event) => {
        if (event.button !== 0) {
          return;
        }

        event.preventDefault();
        startDragging();
      }}
    >
      {itemType === "type1" ? (
        <img
          className="overlay-visual overlay-visual-image"
          src="/img/miserarenaiyo_touka.png"
          alt={overlayLabels[itemType]}
          draggable={false}
        />
      ) : (
        <div
          className="overlay-visual overlay-visual-block"
          role="img"
          aria-label={overlayLabels[itemType]}
        />
      )}
    </main>
  );
}

export default App;
