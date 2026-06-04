# Screenshots & Demo

This folder contains screenshots and demo GIFs used in README and documentation.

## Files

| File | Description | Recommended Size |
|------|-------------|------------------|
| `hero.png` | Main screenshot for README hero section | 1200×800 |
| `search.png` | Search palette in action | 1200×800 |
| `chat-agent.png` | AI assistant with tool calling | 1200×800 |
| `tags.png` | Tag management | 1200×800 |
| `settings.png` | Settings page | 1200×800 |
| `demo.gif` | 30-second demo for README | ≤ 5MB |

## How to Capture

### Screenshots (macOS)
- Full screen: `⌘ + ⇧ + 3`
- Selection: `⌘ + ⇧ + 4`
- Window: `⌘ + ⇧ + 4` then `Space`

Recommended tool: [shottr](https://shottr.cc/) (free, with annotation).

### Demo GIF
1. Open QuickTime Player → File → New Screen Recording
2. Record ~30 seconds covering: search, AI chat, tag/favorite
3. Export as `.mov`
4. Convert to GIF:
   ```bash
   # using ffmpeg
   ffmpeg -i demo.mov -vf "fps=12,scale=800:-1" -loop 0 demo.gif
   ```
   Or use [gifski](https://gif.ski/) for better quality.

## Used In

- `README.md` / `README_EN.md` — hero image, feature showcase
- `docs/index.md` — VitePress home hero
- GitHub Release notes
