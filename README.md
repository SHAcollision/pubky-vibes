# pubky-vibes

**Strictly** vibe coded [Pubky](https://github.com/pubky/pubky-core) projects.

### Contribution guidelines

- Ideally prompted from your phone. No desktop/laptop allowed.
- All cloud coding agent are allowed.
- Make your client test your code before contributing.
- No IDE, no manual edit on commits. No human code allowed.
- Use voice whenever possible. Go on a walk or for lunch while prompting, no prompting from your desk.
- Add shareable links with your prompting and agent logs for others to learn. For example, links like this one: [Codex](https://chatgpt.com/codex/tasks/task_e_68e97ff5b43083298ebefc7e6980c4ef)
- Keep repo AI friendly. Tell your AI to avoid committing `package-lock.json` or `Cargo.lock`.

## Projects

### [Pubky Swiss Knife](pubky-swiss-knife)

A multi-tool for anything Pubky. Built using the Pubky rust SDK and Dioxus. [Initial Codex prompt here](https://chatgpt.com/s/cd_68e9a87740108191936e11721d314fea)
<img width="1210" height="673" alt="image" src="https://github.com/user-attachments/assets/41218313-0177-4134-bc79-d611fbd9399d" />

### [Portable Homeserver](portable-homeserver)

Embedded multiplatform mainnet and testnet homeserver. Built using the Pubky rust SDK and Dioxus. [Initial Codex prompt here](https://chatgpt.com/s/cd_68e9b9732a688191a61e6ff03a49cbdf).
<img width="913" height="782" alt="image" src="https://github.com/user-attachments/assets/e473c194-1b0e-4d9c-84e1-e2b138e063c3" />

## Agent Context

Currently using [microsoft/pragmatic-rust-guidelines](https://microsoft.github.io/rust-guidelines/agents/all.txt) as a base for `AGENTS.md`

Add this on your agent environment (container) setup script.

```bash
# Microsoft's Pragmatic Rust Guidelines for Agents (21K tokens)
curl -fsSL https://microsoft.github.io/rust-guidelines/agents/all.txt >> AGENTS.md

# Ubuntu deps needed for Dioxus
apt update
apt install -y build-essential pkg-config libxdo-dev \
  libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev \
  libssl-dev libayatana-appindicator3-dev
```
