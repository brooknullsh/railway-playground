package main

import (
  "log/slog"
  "os"
  "strings"
  "time"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
  "github.com/lmittmann/tint"
)

func init() {
  var level slog.Level
  envLevel := os.Getenv("LOG_LEVEL")
  switch strings.ToLower(envLevel) {
  case "debug":
    level = slog.LevelDebug
  case "info":
    level = slog.LevelInfo
  case "warn":
    level = slog.LevelWarn
  case "error":
    level = slog.LevelError
  default:
    level = slog.LevelDebug
  }

  var opts tint.Options
  opts.Level = level
  opts.TimeFormat = time.Kitchen

  overwrite := tint.NewHandler(os.Stderr, &opts)
  logger := slog.New(overwrite)
  slog.SetDefault(logger)
}

func main() {
  store, err := store.New()
  if err != nil {
    slog.Error("creating the store", "error", err)
    os.Exit(1)
  }

  app := fiber.New()
  app.State().Set(handler.StoreKey, &store)
  app.Get("/", handler.Index)

  var port string
  if port = os.Getenv("PORT"); port == "" {
    port = ":8080"
  } else {
    port = ":" + port
  }

  var config fiber.ListenConfig
  config.DisableStartupMessage = true

  if err := app.Listen(port, config); err != nil {
    slog.Error("starting server", "error", err)
    os.Exit(1)
  }
}
