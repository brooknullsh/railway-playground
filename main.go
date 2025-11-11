package main

import (
  "context"
  "errors"
  "log/slog"
  "net/http"
  "os"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
)

// See: https://github.com/dunamismax/go-web-server

func initLogger() {
  options := slog.HandlerOptions{Level: slog.LevelDebug}
  handler := slog.NewTextHandler(os.Stdout, &options)

  slog.SetDefault(slog.New(handler))
}

func initStore(ctx context.Context) *store.Store {
  store, err := store.New(ctx, os.Getenv("DATABASE_URL"))
  if err != nil {
    slog.Error("creating store", "error", err)
    os.Exit(1)
  }

  return store
}

func initApp(store *store.Store) *echo.Echo {
  app := echo.New()

  handlers := handler.New(store)
  handlers.Register(app)

  return app
}

func initPort() (port string) {
  if value, ok := os.LookupEnv("PORT"); ok {
    port = ":" + value
  } else {
    port = ":8080"
  }

  return
}

func main() {
  initLogger()
  ctx := context.Background()

  store := initStore(ctx)
  defer store.Database.Close()

  app := initApp(store)
  port := initPort()

  if err := app.Start(port); err != nil && !errors.Is(err, http.ErrServerClosed) {
    slog.Error("starting server", "error", err)
    os.Exit(1)
  }
}
