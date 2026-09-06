"use strict";
document.querySelectorAll("[data-preview]").forEach((button) => {
  button.addEventListener("click", () => {
    const theme = button.dataset.preview;
    document.getElementById("app-preview").src = `assets/app-${theme}.png`;
    document.querySelectorAll("[data-preview]").forEach((item) => {
      item.setAttribute("aria-pressed", String(item === button));
    });
  });
});
