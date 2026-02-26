TYPST = typst
DRAWIO = drawio
SRC_DIR = doc
BUILD_DIR = build
DIAGRAMS_DIR = diagrams
BUILD_DIAGRAMS_DIR = $(BUILD_DIR)/diagrams

# Find all .drawio files
DIAGRAM_FILES := $(wildcard $(DIAGRAMS_DIR)/*.drawio)
# Map to build outputs
DIAGRAMS := $(patsubst $(DIAGRAMS_DIR)/%.drawio,$(BUILD_DIAGRAMS_DIR)/%.pdf,$(DIAGRAM_FILES))

MAIN_TYP = $(SRC_DIR)/ecrts.typ
OUTPUT_PDF = $(BUILD_DIR)/ecrts.pdf

# Find all .drawio files in the diagrams directory
DIAGRAM_FILES = $(wildcard $(DIAGRAMS_DIR)/*.drawio)

all: $(OUTPUT_PDF)

open:
	okular build/main.pdf >/dev/null &

watch:
	@watchexec -w $(DIAGRAMS_DIR) -e drawio -- make & \
	WATCH_PID=$$!; \
	$(TYPST) watch $(MAIN_TYP) $(OUTPUT_PDF) --root .; \
	kill $$WATCH_PID 2>/dev/null

# Build the main PDF
$(OUTPUT_PDF): $(MAIN_TYP) diagrams
	mkdir -p $(BUILD_DIR)
	$(TYPST) compile $(MAIN_TYP) $(OUTPUT_PDF) --root .

# Rebuild diagrams if needed. Switch between batch mode or single export
# depending on how many diagrams we need to rebuild to avoid the expensive
# drawio startup times
diagrams: | $(BUILD_DIAGRAMS_DIR)
	@needed=""
	@for f in $(DIAGRAM_FILES); do \
	  out="$(BUILD_DIAGRAMS_DIR)/$$(basename $$f .drawio).pdf"; \
	  if [ ! -f "$$out" ] || [ "$$f" -nt "$$out" ]; then \
	    needed="$$needed $$f"; \
	  fi; \
	done; \
	count=$$(echo $$needed | wc -w); \
	if [ $$count -gt 1 ]; then \
	  echo "Batch exporting diagrams..."; \
	  $(DRAWIO) $(DIAGRAMS_DIR) --export --format pdf --crop --output $(BUILD_DIAGRAMS_DIR) 2>/dev/null; \
	elif [ $$count -eq 1 ]; then \
	  echo "Exporting $$needed"; \
	  $(DRAWIO) $$needed --export --format pdf --crop --output $(BUILD_DIAGRAMS_DIR) 2>/dev/null; \
	fi

# Ensure the build/diagrams directory exists
$(BUILD_DIAGRAMS_DIR):
	mkdir -p $@

# Clean build artifacts
clean:
	rm -rf $(BUILD_DIR)

.PHONY: all clean watch open diagrams