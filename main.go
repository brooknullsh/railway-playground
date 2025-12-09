package main

import (
  "log/slog"
  "os"
  "time"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
  "github.com/lmittmann/tint"
)

func init() {
  var opts tint.Options
  opts.Level = slog.LevelDebug
  opts.TimeFormat = time.Stamp

  overwrite := tint.NewHandler(os.Stderr, &opts)
  logger := slog.New(overwrite)
  slog.SetDefault(logger)
}

func main() {
  port := ":"
  if port += os.Getenv("PORT"); port == ":" {
    port += "8080"
  }

  store, err := store.New()
  if err != nil {
    slog.Error("[STORE] creating and connecting to store", "error", err)
    os.Exit(1)
  }

  app := fiber.New()
  handler.Setup(app, &store)

  if err := app.Listen(port); err != nil {
    slog.Error("[CRASH] runtime server failure", "error", err)
    store.Close()
  }
}
