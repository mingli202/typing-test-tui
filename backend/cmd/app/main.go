package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"net/http"
	"os/signal"
	"syscall"
	"tui/backend/internal/handlers/hub"
	"tui/backend/internal/services/data_provider"
	"tui/backend/internal/services/name_provider"
)

const port = 8080

func main() {
	quitCtx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	mux := http.NewServeMux()

	err := registerRoutes(mux, quitCtx)

	if err != nil {
		log.Fatal(err)
	}

	server := &http.Server{
		Addr:    fmt.Sprintf(":%v", port),
		Handler: mux,
		BaseContext: func(l net.Listener) context.Context {
			ctx := context.WithValue(quitCtx, "serverAddr", l.Addr().String())
			return ctx
		},
	}

	go func() {
		log.Printf("Starting server on port %v\n", port)
		err := server.ListenAndServe()

		if err != nil {
			log.Println(err)
		}
	}()

	<-quitCtx.Done()

	ctx := context.Background()

	if err := server.Shutdown(ctx); err != nil {
		log.Printf("Server forced to shut down, %s\n", err)
	}

	log.Println("Server stopped")
}

func registerRoutes(mux *http.ServeMux, ctx context.Context) error {
	dataProvider, err := data_provider.NewDataProvider()

	if err != nil {
		return err
	}

	nameProvider, err := name_provider.NewNameProvider()

	if err != nil {
		return err
	}

	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		fmt.Fprintf(w, "Ready at %v!\n", ctx.Value("serverAddr"))
	})
	mux.Handle("/ws", hub.Handler(&dataProvider, &nameProvider, ctx))
	mux.HandleFunc("/new_data", func(w http.ResponseWriter, r *http.Request) {
		data, err := dataProvider.NewData()

		if err != nil {
			http.Error(w, "failed to load data", http.StatusInternalServerError)
			return
		}

		p, err := json.Marshal(data)

		if err != nil {
			http.Error(w, "failed to serialize data", http.StatusInternalServerError)
			return
		}

		_, _ = w.Write(p)
	})

	return nil
}
