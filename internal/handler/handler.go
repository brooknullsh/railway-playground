package handler

import (
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
)

func InitialiseWithState(app *fiber.App, store *store.Store) {
  index := IndexHandler{store}

  app.Get("/", index.Root)
}

type IndexHandler struct {
  store *store.Store
}

func (this *IndexHandler) Root(ctx fiber.Ctx) error {
  user, code := this.store.User.GetByName(ctx, "Alice")
  if code != http.StatusOK {
    return ctx.SendStatus(code)
  }

  return ctx.SendString(user.FirstName)
}
