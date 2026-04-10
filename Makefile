.PHONY: build test install clean build-sidebar install-sidebar

BINARY_NAME := amux
SIDEBAR_NAME := amux-sidebar
BUILD_DIR := .
INSTALL_DIR := $(HOME)/bin

# Build the main binary
build:
	go build -o $(BUILD_DIR)/$(BINARY_NAME) ./cmd/amux

# Build the sidebar binary
build-sidebar:
	go build -o $(BUILD_DIR)/$(SIDEBAR_NAME) ./cmd/amux-sidebar

# Build both binaries
build-all: build build-sidebar

# Run tests
test:
	go test -v ./...

# Install main binary
install: build
	@echo "Installing $(BINARY_NAME) to $(INSTALL_DIR)..."
	@mkdir -p $(INSTALL_DIR)
	@cp $(BUILD_DIR)/$(BINARY_NAME) $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Installation complete!"
	@echo "Make sure $(INSTALL_DIR) is in your PATH"

# Install sidebar binary
install-sidebar: build-sidebar
	@echo "Installing $(SIDEBAR_NAME) to $(INSTALL_DIR)..."
	@mkdir -p $(INSTALL_DIR)
	@cp $(BUILD_DIR)/$(SIDEBAR_NAME) $(INSTALL_DIR)/$(SIDEBAR_NAME)
	@echo "Installation complete!"

# Install both binaries
install-all: install install-sidebar

# Clean build artifacts
clean:
	rm -f $(BUILD_DIR)/$(BINARY_NAME) $(BUILD_DIR)/$(SIDEBAR_NAME)

# Development helpers
dev: build
	./$(BINARY_NAME) start

init: build
	./$(BINARY_NAME) init
