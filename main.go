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
    slog.Error("[STORE]" + err.Error())
    os.Exit(1)
  }

  return store
}

func initApp(store *store.Store) (app *echo.Echo, port string) {
  app = echo.New()

  handlers := handler.New(store)
  handlers.Register(app)

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

  app, port := initApp(store)
  if err := app.Start(port); err != nil && !errors.Is(err, http.ErrServerClosed) {
    slog.Error("[APP]" + err.Error())
    os.Exit(1)
  }
}
