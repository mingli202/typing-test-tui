package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"net/http"
)

const port = 8080

func main() {
	mux := http.NewServeMux()

	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		fmt.Fprintf(w, "Ready at %v!\n", ctx.Value("serverAddr"))
	})

	ctx, cancelCtx := context.WithCancel(context.Background())

	server := &http.Server{
		Addr:    fmt.Sprintf(":%v", port),
		Handler: mux,
		BaseContext: func(l net.Listener) context.Context {
			ctx = context.WithValue(ctx, "serverAddr", l.Addr().String())
			return ctx
		},
	}

	go func() {
		defer cancelCtx()
		log.Printf("Starting server on port %v\n", port)
		err := server.ListenAndServe()

		if err != nil {
			log.Fatal(err)
		}
	}()

	<-ctx.Done()
}
