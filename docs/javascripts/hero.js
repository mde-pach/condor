// Home hero: replay the animation, copy the install command.
(function () {
  var hero = document.querySelector(".ch-hero");
  if (!hero) return;
  var replay = hero.querySelector(".ch-replay");
  replay.addEventListener("click", function () {
    hero.classList.remove("ch-play");
    void hero.offsetWidth; // restart the CSS animations
    hero.classList.add("ch-play");
  });
  var copy = hero.querySelector(".ch-cmd button");
  var cmd = hero.querySelector(".ch-cmd code");
  copy.addEventListener("click", function () {
    var done = function () {
      copy.textContent = "Copied";
      setTimeout(function () { copy.textContent = "Copy"; }, 1500);
    };
    try {
      navigator.clipboard.writeText(cmd.textContent).then(done, function () {});
    } catch (e) { /* the command stays selectable */ }
  });
})();
