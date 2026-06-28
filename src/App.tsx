import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type OverlayItemType = "type1" | "type2";
type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

const overlayLabels: Record<OverlayItemType, string> = {
  type1: "Type1",
  type2: "Type2",
};

const resizeHandleDirections: Array<{
  className: string;
  direction: ResizeDirection;
  label: string;
}> = [
  { className: "overlay-resize-handle--nw", direction: "NorthWest", label: "Resize top left" },
  { className: "overlay-resize-handle--ne", direction: "NorthEast", label: "Resize top right" },
  { className: "overlay-resize-handle--sw", direction: "SouthWest", label: "Resize bottom left" },
  { className: "overlay-resize-handle--se", direction: "SouthEast", label: "Resize bottom right" },
];

function App() {
  const [itemType, setItemType] = useState<OverlayItemType>("type1");

  const startDragging = () => {
    void getCurrentWindow().startDragging();
  };

  const startResizing = (direction: ResizeDirection) => {
    void getCurrentWindow().startResizeDragging(direction);
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
      {resizeHandleDirections.map(({ className, direction, label }) => (
        <button
          key={direction}
          type="button"
          className={`overlay-resize-handle ${className}`}
          aria-label={label}
          onMouseDown={(event) => {
            if (event.button !== 0) {
              return;
            }

            event.preventDefault();
            event.stopPropagation();
            startResizing(direction);
          }}
        />
      ))}
    </main>
  );
}

export default App;
