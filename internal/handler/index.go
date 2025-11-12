package handler

import (
  "context"
  "log/slog"
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/jackc/pgx/v5"
  "github.com/labstack/echo/v4"
)

type IndexHandler struct {
  store *store.Store
}

type User struct {
  Id        int    `db:"id" json:"id"`
  Age       int    `db:"age" json:"age"`
  IsPro     bool   `db:"is_pro" json:"isPro"`
  Mobile    string `db:"mobile" json:"mobile"`
  LastName  string `db:"last_name" json:"lastName"`
  FirstName string `db:"first_name" json:"firstName"`
}

func (h *IndexHandler) Root(ctx echo.Context) error {
  rows, err := h.store.Database.Query(context.Background(), "SELECT * FROM users")
  if err != nil {
    slog.Error("[QUERYING]" + err.Error())
    return ctx.NoContent(http.StatusInternalServerError)
  }

  defer rows.Close()

  users, err := pgx.CollectRows(rows, pgx.RowToStructByName[User])
  if err != nil {
    slog.Error("[MAPPING]" + err.Error())
    return ctx.NoContent(http.StatusInternalServerError)
  }

  return ctx.JSON(http.StatusOK, users)
}
