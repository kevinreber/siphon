-- Siphon Neovim Plugin
-- Auto-load entry point

-- Prevent double loading
if vim.g.loaded_siphon then
  return
end
vim.g.loaded_siphon = true

-- Create user commands
vim.api.nvim_create_user_command("SiphonStatus", function()
  require("siphon").status()
end, { desc = "Check Siphon daemon status" })

vim.api.nvim_create_user_command("SiphonPause", function()
  require("siphon").pause()
end, { desc = "Pause Siphon tracking" })

vim.api.nvim_create_user_command("SiphonResume", function()
  require("siphon").resume()
end, { desc = "Resume Siphon tracking" })

vim.api.nvim_create_user_command("SiphonToggle", function()
  require("siphon").toggle()
end, { desc = "Toggle Siphon tracking" })

vim.api.nvim_create_user_command("SiphonTrack", function(opts)
  local event_type = opts.args ~= "" and opts.args or "manual"
  require("siphon").track_event(event_type)
end, {
  nargs = "?",
  desc = "Manually track an event",
})
