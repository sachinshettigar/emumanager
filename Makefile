# Thin delegator to `just` for environments without it on PATH.
# Prefer `just <recipe>` directly. See justfile / AGENTS.md §4.

JUST := $(shell command -v just 2>/dev/null)

.PHONY: setup dev check-fast validate progress bindings test e2e
setup dev check-fast validate progress bindings test e2e:
ifeq ($(JUST),)
	@echo "just not found — install from https://just.systems, or run scripts/ directly"; exit 1
else
	@just $@
endif
