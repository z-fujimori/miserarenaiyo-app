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

const OVERLAY_LABEL = "overlay";
const overlayLabels: Record<OverlayItemType, string> = {
  type1: "Type1",
  type2: "Type2",
};

const compactOverlayLabels: Record<OverlayItemType, string> = {
  type1: "1",
  type2: "2",
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

function setWindowKind(kind: "overlay" | "settings") {
  document.body.dataset.windowKind = kind;
}

function OverlayApp() {
  const [itemType, setItemType] = useState<OverlayItemType>("type1");

  const startDragging = () => {
    void getCurrentWindow().startDragging();
  };

  const startResizing = (direction: ResizeDirection) => {
    void getCurrentWindow().startResizeDragging(direction);
  };

  useEffect(() => {
    setWindowKind("overlay");

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

function SettingsApp() {
  const [itemType, setItemType] = useState<OverlayItemType>("type1");
  const [overlayVisible, setOverlayVisible] = useState(false);

  const hideWindow = () => {
    void getCurrentWindow().hide();
  };

  useEffect(() => {
    setWindowKind("settings");

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        hideWindow();
      }
    };

    window.addEventListener("keydown", onKeyDown);

    void invoke<OverlayItemType>("get_overlay_item_type").then((value) => {
      setItemType(value);
    });
    void invoke<boolean>("is_overlay_visible").then((value) => {
      setOverlayVisible(value);
    });

    let unlisten: (() => void) | undefined;
    void listen<OverlayItemType>("overlay-item-type-changed", (event) => {
      setItemType(event.payload);
    }).then((dispose) => {
      unlisten = dispose;
    });

    let unlistenVisibility: (() => void) | undefined;
    void listen<boolean>("overlay-visibility-changed", (event) => {
      setOverlayVisible(event.payload);
    }).then((dispose) => {
      unlistenVisibility = dispose;
    });

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      unlisten?.();
      unlistenVisibility?.();
      if (document.body.dataset.windowKind === "settings") {
        delete document.body.dataset.windowKind;
      }
    };
  }, []);

  return (
    <main className="settings-root">
      <section className="settings-shell">
        <header className="settings-hero">
          <div className="settings-hero__copy">
            <p className="settings-title">Miserarenaiyo</p>
          </div>
          <div className="settings-status" aria-label="現在の選択状態">
            <strong className="settings-status__value">{compactOverlayLabels[itemType]}</strong>
          </div>
        </header>

        <section className="settings-card">
          <div className="settings-actions">
            <button
              type="button"
              className="settings-button"
              onClick={() => {
                void invoke("toggle_overlay");
              }}
            >
              {overlayVisible ? "Hide Overlay" : "Show Overlay"}
            </button>
            <button
              type="button"
              className="settings-button"
              onClick={() => {
                void invoke("set_type1");
              }}
            >
              1
            </button>
            <button
              type="button"
              className="settings-button"
              onClick={() => {
                void invoke("set_type2");
              }}
            >
              2
            </button>
          </div>
        </section>
      </section>
    </main>
  );
}

function App() {
  const windowLabel = getCurrentWindow().label;

  if (windowLabel === OVERLAY_LABEL) {
    return <OverlayApp />;
  }

  return <SettingsApp />;
}

export default App;
