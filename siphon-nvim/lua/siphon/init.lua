-- Siphon Neovim Plugin
-- Captures editor events and sends them to the Siphon daemon

local M = {}

-- Configuration with defaults
M.config = {
  -- Siphon daemon URL
  daemon_url = "http://127.0.0.1:9847",
  -- Enable/disable event types
  track_file_open = true,
  track_file_save = true,
  track_file_close = true,
  track_buffer_enter = true,
  track_insert_leave = false, -- Can be noisy
  -- Debounce time in milliseconds
  debounce_ms = 1000,
  -- Debug mode
  debug = false,
}

-- State
local last_event_time = {}
local curl_available = nil

-- Check if curl is available
local function check_curl()
  if curl_available ~= nil then
    return curl_available
  end
  local handle = io.popen("which curl 2>/dev/null")
  if handle then
    local result = handle:read("*a")
    handle:close()
    curl_available = result ~= ""
  else
    curl_available = false
  end
  return curl_available
end

-- Debug log
local function debug_log(msg)
  if M.config.debug then
    vim.notify("[Siphon] " .. msg, vim.log.levels.DEBUG)
  end
end

-- Send event to daemon (fire-and-forget)
local function send_event(event_type, data)
  if not check_curl() then
    debug_log("curl not available, skipping event")
    return
  end

  -- Debounce events
  local key = event_type .. ":" .. (data.file_path or "")
  local now = vim.loop.now()
  if last_event_time[key] and (now - last_event_time[key]) < M.config.debounce_ms then
    debug_log("Debounced: " .. event_type)
    return
  end
  last_event_time[key] = now

  -- Build JSON payload
  local payload = vim.fn.json_encode({
    action = event_type,
    file_path = data.file_path or "",
    language = data.language,
    lines_changed = data.lines_changed,
  })

  -- Fire-and-forget curl request
  local cmd = string.format(
    "curl -s -X POST -H 'Content-Type: application/json' -d '%s' '%s/events/editor' >/dev/null 2>&1 &",
    payload:gsub("'", "'\\''"),
    M.config.daemon_url
  )

  os.execute(cmd)
  debug_log("Sent: " .. event_type .. " for " .. (data.file_path or "unknown"))
end

-- Get file info
local function get_file_info(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  local file_path = vim.api.nvim_buf_get_name(bufnr)
  local filetype = vim.bo[bufnr].filetype
  local lines = vim.api.nvim_buf_line_count(bufnr)

  return {
    file_path = file_path ~= "" and file_path or nil,
    language = filetype ~= "" and filetype or nil,
    lines_changed = lines,
  }
end

-- Event handlers
local function on_file_open(bufnr)
  if not M.config.track_file_open then return end
  local info = get_file_info(bufnr)
  if info.file_path then
    send_event("file_open", info)
  end
end

local function on_file_save(bufnr)
  if not M.config.track_file_save then return end
  local info = get_file_info(bufnr)
  if info.file_path then
    send_event("file_save", info)
  end
end

local function on_file_close(bufnr)
  if not M.config.track_file_close then return end
  local info = get_file_info(bufnr)
  if info.file_path then
    send_event("file_close", info)
  end
end

local function on_buffer_enter(bufnr)
  if not M.config.track_buffer_enter then return end
  local info = get_file_info(bufnr)
  if info.file_path then
    send_event("buffer_enter", info)
  end
end

local function on_insert_leave(bufnr)
  if not M.config.track_insert_leave then return end
  local info = get_file_info(bufnr)
  if info.file_path then
    send_event("edit", info)
  end
end

-- Setup function
function M.setup(opts)
  -- Merge user config with defaults
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})

  -- Create autocommand group
  local group = vim.api.nvim_create_augroup("Siphon", { clear = true })

  -- File open (BufReadPost)
  vim.api.nvim_create_autocmd("BufReadPost", {
    group = group,
    callback = function(args)
      on_file_open(args.buf)
    end,
  })

  -- File save (BufWritePost)
  vim.api.nvim_create_autocmd("BufWritePost", {
    group = group,
    callback = function(args)
      on_file_save(args.buf)
    end,
  })

  -- File close (BufDelete)
  vim.api.nvim_create_autocmd("BufDelete", {
    group = group,
    callback = function(args)
      on_file_close(args.buf)
    end,
  })

  -- Buffer enter
  vim.api.nvim_create_autocmd("BufEnter", {
    group = group,
    callback = function(args)
      on_buffer_enter(args.buf)
    end,
  })

  -- Insert leave (edits)
  vim.api.nvim_create_autocmd("InsertLeave", {
    group = group,
    callback = function(args)
      on_insert_leave(args.buf)
    end,
  })

  debug_log("Siphon plugin initialized")
end

-- Manual event triggers
function M.track_event(event_type, data)
  send_event(event_type, data or get_file_info())
end

-- Check daemon status
function M.status()
  local handle = io.popen(string.format(
    "curl -s '%s/health' 2>/dev/null",
    M.config.daemon_url
  ))
  if handle then
    local result = handle:read("*a")
    handle:close()
    if result and result ~= "" then
      local ok, json = pcall(vim.fn.json_decode, result)
      if ok and json and json.status == "ok" then
        vim.notify("[Siphon] Daemon is running (v" .. (json.version or "unknown") .. ")", vim.log.levels.INFO)
        return true
      end
    end
  end
  vim.notify("[Siphon] Daemon is not running", vim.log.levels.WARN)
  return false
end

-- Pause/resume tracking
M.paused = false

function M.pause()
  M.paused = true
  vim.notify("[Siphon] Tracking paused", vim.log.levels.INFO)
end

function M.resume()
  M.paused = false
  vim.notify("[Siphon] Tracking resumed", vim.log.levels.INFO)
end

function M.toggle()
  if M.paused then
    M.resume()
  else
    M.pause()
  end
end

return M
