package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"net/http"
)

type GameHandler struct{}

func (GameHandler) ServeHTTP(http.ResponseWriter, *http.Request) {}

func main() {
	mux := http.NewServeMux()

	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		fmt.Fprintf(w, "Ready at %v!\n", ctx.Value("serverAddr"))
	})

	ctx, cancelCtx := context.WithCancel(context.Background())

	server := &http.Server{
		Addr:    ":8080",
		Handler: mux,
		BaseContext: func(l net.Listener) context.Context {
			ctx = context.WithValue(ctx, "serverAddr", l.Addr().String())
			return ctx
		},
	}

	go func() {
		log.Fatal(server.ListenAndServe())
		cancelCtx()
	}()

	<-ctx.Done()
}
