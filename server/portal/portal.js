// Guest page (Spec §4). The capability token arrives in the URL fragment,
// which browsers never send to the server. It is taken out of the address
// bar and history before anything else happens, kept only in memory, and
// exchanged once for a short-lived cookie session.
"use strict";
(function () {
  var token = location.hash.length > 1 ? location.hash.slice(1) : "";
  history.replaceState(null, "", location.pathname);

  var $ = function (id) { return document.getElementById(id); };
  function show(id) {
    ["loading", "gone", "offline", "form", "thanks"].forEach(function (s) { $(s).hidden = s !== id; });
  }

  function exchange() {
    show("loading");
    if (!/^[A-Za-z0-9_-]{43}$/.test(token)) { show("gone"); return; }
    fetch("/api/v1/guest/session", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      body: JSON.stringify({ token: token })
    }).then(function (r) {
      if (r.status === 404) { token = ""; show("gone"); return null; }
      if (!r.ok) { throw new Error("status " + r.status); }
      token = ""; // exchanged; the cookie session carries on
      return r.json();
    }).then(function (view) {
      if (!view) return;
      $("dish").textContent = view.dishName || "Your dish";
      $("place").textContent = view.locationName;
      if (view.hasPhoto) { $("photo").src = "/api/v1/guest/photo"; $("photo").hidden = false; }
      show("form");
    }).catch(function () { show("offline"); });
  }

  $("retry").addEventListener("click", exchange);

  var form = $("form");
  form.addEventListener("change", function () {
    $("send").disabled = !form.querySelector("input[name=rating]:checked");
  });
  $("comment").addEventListener("input", function () { $("count").textContent = String($("comment").value.length); });

  form.addEventListener("submit", function (e) {
    e.preventDefault();
    var picked = form.querySelector("input[name=rating]:checked");
    if (!picked) return;
    $("send").disabled = true;
    $("error").hidden = true;
    var comment = $("comment").value.trim();
    fetch("/api/v1/guest/feedback", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      body: JSON.stringify({ rating: Number(picked.value), comment: comment || null })
    }).then(function (r) {
      if (r.status === 201) { show("thanks"); return; }
      if (r.status === 404) { show("gone"); return; }
      return r.json().then(function (body) { throw new Error(body.message || "could not send"); });
    }).catch(function (err) {
      $("error").textContent = err.message || "Could not send. Try again.";
      $("error").hidden = false;
      $("send").disabled = false;
    });
  });

  exchange();
})();
