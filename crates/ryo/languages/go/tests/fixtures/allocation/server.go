package server

import (
	"context"
	"errors"
	"log"
	"net/http"
	"time"
)

// Serve runs handler on addr until ctx is done.
func Serve(ctx context.Context, addr string, handler http.Handler) error {
	srv := &http.Server{Addr: addr, Handler: handler, ReadHeaderTimeout: 5 * time.Second}
	errs := make(chan error, 1)
	go func() {
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			errs <- err
		}
		close(errs)
	}()
	select {
	case <-ctx.Done():
		log.Println("shutting down")
		return srv.Shutdown(context.Background())
	case err := <-errs:
		return err
	}
}
