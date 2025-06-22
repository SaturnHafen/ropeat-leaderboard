const leaderboard_ids = ["nickname"];
const hpi_ids = ["email-hpi", "name-hpi"];

document.addEventListener("DOMContentLoaded", () => {
  document
    .getElementById("wants-leaderboard")
    .addEventListener("change", function (x) {
      leaderboard_ids.forEach((ids) => {
        document.getElementById(ids).required = this.checked;
      });
    });

  document.getElementById("wants-hpi").addEventListener("change", function (x) {
    hpi_ids.forEach((ids) => {
      document.getElementById(ids).required = this.checked;
    });
  });
});
