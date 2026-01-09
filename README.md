# themalingadingdong

Base24 color palette generator using Hellwig-Fairchild CAM16 color appearance model with APCA contrast validation. Outputs [tinted-theming](https://github.com/tinted-theming/home) compatible schemes.

## Features

- **Perceptually uniform accents**: Uses Hellwig-Fairchild CAM16 (JMh) for accurate color appearance
- **APCA contrast validation**: Ensures accessibility per WCAG 3.0 guidelines
- **Helmholtz-Kohlrausch effect**: Accounts for chromatic brightness perception
- **Interactive TUI**: Real-time preview with parameter adjustment
- **Import/Export**: Load existing Base16/Base24 schemes, output YAML or JSON

## Installation

```bash
cargo install --path .
```

## Usage

### Generate a dark theme

```bash
themalingadingdong -b "#1d2021" -f "#ebdbb2" --name "gruvbox-dark"
```

### Generate a light theme

```bash
themalingadingdong -b "#fbf1c7" -f "#3c3836" --name "gruvbox-light"
```

### Interactive mode

```bash
themalingadingdong -b "#000000" -f "#ffffff" --name "my-theme" -i
```

### Import and edit existing scheme

```bash
themalingadingdong --input scheme.yaml -i
```

### Generate both variants

```bash
themalingadingdong -b "#1d2021" -f "#ebdbb2" --name "gruvbox" --variant both -o gruvbox
```

### JSON output

```bash
themalingadingdong -b "#282828" -f "#ebdbb2" --name "theme" --format json
```

## Color Input

Accepts any CSS color format via `csscolorparser`:

- Hex: `#1d2021`, `1d2021`
- RGB: `rgb(29, 32, 33)`, `rgba(29, 32, 33, 1)`
- HSL: `hsl(195, 6%, 12%)`
- OKLCH: `oklch(0.25 0.01 240)`
- Named: `black`, `rebeccapurple`

## Configuration

Save/load TOML configuration files:

```bash
# Save current settings
themalingadingdong -b "#000" -f "#fff" --name "dark" --save-config dark.toml

# Load from config
themalingadingdong --config dark.toml
```

## Accent Optimization

The solver optimizes accent colors (base08-base0F, base10-base17) for:

- **Uniform lightness**: All accents share similar perceived brightness
- **Minimum contrast**: Guaranteed APCA floor against background
- **Maximum colorfulness**: Vibrant colors within sRGB gamut

Key parameters:

| Flag | Description | Default |
|------|-------------|---------|
| `--min-contrast` | APCA floor for base08-base0F | 45 |
| `--extended-min-contrast` | APCA floor for base10-base17 | 60 |
| `--target-j` | Target lightness (J') | 65 |
| `--target-m` | Target colorfulness (M) | 40 |
| `--j-weight` | Uniformity vs vibrancy (0-1) | 0.5 |

## Hue Overrides

Customize accent hues (in degrees):

```bash
themalingadingdong -b "#000" -f "#fff" --name "theme" \
  --hue-08 30 \   # Red
  --hue-0b 120    # Green
```

Default hues: Red=25, Orange=55, Yellow=90, Green=145, Cyan=180, Blue=250, Purple=285, Magenta=335

## Shell Completions

```bash
themalingadingdong --completions bash > ~/.local/share/bash-completion/completions/themalingadingdong
themalingadingdong --completions zsh > ~/.zfunc/_themalingadingdong
themalingadingdong --completions fish > ~/.config/fish/completions/themalingadingdong.fish
```

## Architecture

```
Input (CSS colors)
    |
    v
Hellwig-Fairchild CAM16 (JMh)  <-- Perceptually uniform color space
    |
    v
COBYLA Solver                   <-- Multi-objective optimization
    |                               (contrast, uniformity, colorfulness)
    v
APCA Validation                 <-- Accessibility verification
    |
    v
Base24 Scheme (YAML/JSON)       <-- tinted-theming compatible output
```

## License

MIT
