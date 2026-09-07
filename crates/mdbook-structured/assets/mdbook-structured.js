(() => {
  const dismissTooltips = [];
  document
    .querySelectorAll(".structured-document [data-structured-action]")
    .forEach((button) => {
      let timer;
      const close = () => {
        clearTimeout(timer);
        button.removeAttribute("data-structured-tooltip-open");
      };
      const open = () => {
        dismissTooltips.forEach((dismiss) => dismiss());
        button.setAttribute("data-structured-tooltip-open", "");
      };
      dismissTooltips.push(close);

      button.addEventListener("pointerenter", (event) => {
        if (event.pointerType !== "touch") timer = setTimeout(open, 400);
      });
      button.addEventListener("pointerleave", () => {
        if (!button.matches(":focus-visible")) close();
      });
      button.addEventListener("focus", () => {
        if (button.matches(":focus-visible")) open();
      });
      button.addEventListener("blur", close);
      button.addEventListener("click", close);
    });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") dismissTooltips.forEach((dismiss) => dismiss());
  });

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
