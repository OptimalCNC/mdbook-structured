(() => {
  document.querySelectorAll(".structured-document").forEach((root) => {
    root.addEventListener("click", (event) => {
      const action = event.target.closest("[data-structured-action]");
      if (!action || action.closest(".structured-document") !== root) {
        return;
      }

      const isExpanded = action.dataset.structuredAction === "expand-all";
      const isCollapsed = action.dataset.structuredAction === "collapse-all";
      if (!isExpanded && !isCollapsed) {
        return;
      }

      root
        .querySelectorAll("details[data-structured-container]")
        .forEach((container) => {
          if (container.closest(".structured-document") === root) {
            container.open = isExpanded;
          }
        });
    });
  });
})();
