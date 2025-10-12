pub const APP_STYLE: &str = r#"
:root {
    color-scheme: dark;
    font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background-color: #05070f;
    color: #f8fbff;
}

body {
    margin: 0;
}

.app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
}

header {
    padding: 24px 32px 16px;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    background: linear-gradient(180deg, rgba(9,16,32,0.9), rgba(5,7,15,0.6));
    border-bottom: 1px solid rgba(124, 208, 255, 0.2);
    box-shadow: 0 18px 36px rgba(5, 7, 15, 0.6);
}

header .branding {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

header .title {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
}

header .subtitle {
    font-size: 16px;
    color: rgba(240, 248, 255, 0.72);
}

main {
    display: grid;
    grid-template-columns: 360px 1fr;
    gap: 24px;
    padding: 24px 32px 48px;
}

.sidebar {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.content {
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.panel {
    background: rgba(13, 20, 42, 0.9);
    border-radius: 16px;
    border: 1px solid rgba(124, 208, 255, 0.18);
    padding: 18px 20px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04), 0 16px 42px rgba(2, 8, 22, 0.65);
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.panel h2 {
    margin: 0;
    font-size: 20px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
}

.panel p {
    margin: 0;
    color: rgba(232, 244, 255, 0.7);
    font-size: 14px;
}

input, textarea, select {
    background: rgba(7, 12, 27, 0.85);
    border: 1px solid rgba(124, 208, 255, 0.4);
    border-radius: 10px;
    padding: 10px 12px;
    color: #f8fbff;
    font-size: 14px;
    font-family: inherit;
}

textarea {
    min-height: 120px;
    resize: vertical;
}

button {
    background: linear-gradient(120deg, #45c9ff, #2f70ff);
    border: none;
    border-radius: 999px;
    padding: 10px 18px;
    color: #05070f;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    cursor: pointer;
    transition: transform 160ms ease, box-shadow 160ms ease;
}

button.secondary {
    background: rgba(124, 208, 255, 0.1);
    color: rgba(240, 248, 255, 0.8);
    border: 1px solid rgba(124, 208, 255, 0.32);
}

button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 10px 20px rgba(69, 201, 255, 0.25);
}

.field-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 12px 16px;
}

.log-feed {
    max-height: 200px;
    overflow-y: auto;
    background: rgba(4, 8, 20, 0.65);
    border-radius: 12px;
    border: 1px solid rgba(69, 201, 255, 0.24);
    padding: 12px 14px;
    font-family: 'JetBrains Mono', 'SFMono-Regular', monospace;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.log-line {
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.log-line .ts {
    color: rgba(124, 208, 255, 0.7);
    font-size: 11px;
}

.log-line.info { color: rgba(240, 248, 255, 0.72); }
.log-line.success { color: rgba(156, 255, 196, 0.86); }
.log-line.warn { color: rgba(255, 204, 138, 0.9); }
.log-line.error { color: rgba(255, 156, 156, 0.88); }

.match-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 16px;
}

.turn-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 320px;
    overflow-y: auto;
}

.turn-card {
    background: rgba(8, 12, 25, 0.9);
    border-radius: 12px;
    border: 1px solid rgba(124, 208, 255, 0.18);
    padding: 12px 14px;
}

.turn-card h4 {
    margin: 0 0 8px;
    font-size: 14px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
}

.turn-card pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12px;
}

.scene-preview {
    aspect-ratio: 16 / 10;
    border-radius: 12px;
    border: 1px solid rgba(124, 208, 255, 0.18);
    background: radial-gradient(circle at 30% 20%, rgba(124, 208, 255, 0.24), transparent),
                radial-gradient(circle at 70% 30%, rgba(69, 201, 255, 0.18), transparent),
                rgba(7, 10, 21, 0.9);
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(240, 248, 255, 0.6);
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
}
"#;
