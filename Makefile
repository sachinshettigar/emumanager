# Thin delegator to `just` for environments without it on PATH.
# Prefer `just <recipe>` directly. See justfile / AGENTS.md §4.

JUST := $(shell command -v just 2>/dev/null)

TARGETS := setup dev check-fast validate progress bindings db-migrate db-prepare \
           test test-rust test-web test-integration e2e schema-check

.PHONY: $(TARGETS)
$(TARGETS):
ifeq ($(JUST),)
	@echo "just not found — install from https://just.systems, or run scripts/ directly"; exit 1
else
	@just $@
endif
