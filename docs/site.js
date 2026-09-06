"use strict";
// Il percorso non viene ricostruito ma riscritto sull'immagine corrente: la
// pagina inglese sta in una sottocartella, e un "assets/..." fisso la
// romperebbe.
document.querySelectorAll("[data-preview]").forEach((button) => {
  button.addEventListener("click", () => {
    const anteprima = document.getElementById("app-preview");
    anteprima.src = anteprima.src.replace(/app-(light|dark)\.png/, `app-${button.dataset.preview}.png`);
    document.querySelectorAll("[data-preview]").forEach((item) => {
      item.setAttribute("aria-pressed", String(item === button));
    });
  });
});
