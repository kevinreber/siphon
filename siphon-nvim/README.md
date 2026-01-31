# Siphon Neovim Plugin

Captures editor events from Neovim and sends them to the Siphon daemon for activity tracking.

## Installation

### Using lazy.nvim

```lua
{
  "siphon-dev/siphon",
  config = function()
    require("siphon").setup({
      -- Optional configuration
      daemon_url = "http://127.0.0.1:9847",
      track_file_open = true,
      track_file_save = true,
      track_file_close = true,
      track_buffer_enter = true,
      track_insert_leave = false,
      debounce_ms = 1000,
      debug = false,
    })
  end,
}
```

### Using packer.nvim

```lua
use {
  "siphon-dev/siphon",
  config = function()
    require("siphon").setup()
  end,
}
```

### Manual Installation

Copy the `siphon-nvim` directory to your Neovim plugins directory:

```bash
cp -r siphon-nvim ~/.local/share/nvim/site/pack/plugins/start/siphon
```

## Configuration

```lua
require("siphon").setup({
  -- Siphon daemon URL (default: localhost)
  daemon_url = "http://127.0.0.1:9847",

  -- Event tracking options
  track_file_open = true,      -- Track when files are opened
  track_file_save = true,      -- Track when files are saved
  track_file_close = true,     -- Track when files are closed
  track_buffer_enter = true,   -- Track buffer switches
  track_insert_leave = false,  -- Track edits (can be noisy)

  -- Debounce rapid events (milliseconds)
  debounce_ms = 1000,

  -- Enable debug logging
  debug = false,
})
```

## Commands

- `:SiphonStatus` - Check if the Siphon daemon is running
- `:SiphonPause` - Pause event tracking
- `:SiphonResume` - Resume event tracking
- `:SiphonToggle` - Toggle tracking on/off
- `:SiphonTrack [event_type]` - Manually track an event

## Tracked Events

| Event | Description |
|-------|-------------|
| `file_open` | File opened in buffer |
| `file_save` | File saved |
| `file_close` | Buffer closed |
| `buffer_enter` | Switched to a buffer |
| `edit` | Made changes (insert mode exit) |

## Requirements

- Neovim 0.7+ (for Lua API support)
- `curl` command available in PATH
- Siphon daemon running on localhost:9847

## License

MIT
