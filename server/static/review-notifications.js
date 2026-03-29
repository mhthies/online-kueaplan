function initializeReviewNotifications(apiEndpoint) {
    const reviewAreaButton = document.getElementById("review-area-button");
    const SESSION_STORAGE_KEY = "reviewNotifications";
    let abortController = null;
    // used for deciding whether to animate change of badge
    let previousCount = 0;

    function init() {
        const storedDataJson = sessionStorage.getItem(SESSION_STORAGE_KEY);
        if (storedDataJson !== null) {
            const storedData = JSON.parse(storedDataJson);
            updateReviewButton(storedData.to_review, false);
        }
        setInterval(doUpdate, 30000);
        doUpdate();
    }

    function doUpdate() {
        fetchReviewNotificationsFromApi().then((data) => {
            sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(data));
            updateReviewButton(data.to_review, true);
        });
    }

    async function fetchReviewNotificationsFromApi() {
        if (abortController !== null) {
            abortController.abort();
        }
        abortController = new AbortController();
        return window.fetch(apiEndpoint,
            {
                "signal": abortController.signal
            })
            .catch((err) => {
                if (err instanceof DOMException && err.name === "AbortError") {
                    console.log("Running fetch has been aborted");
                    return;
                }
                console.error("Failed to fetch review notifications: ", err);
            })
            .then(async (response) => {
                if (!response.ok) {
                    console.warn("Failed to fetch review notifications: HTTP " + response.status + ": " + await response.text());
                    return;
                }
                return response.json();
            });
    }

    function updateReviewButton(count, animate) {
        let counterBadge;
        let badges = reviewAreaButton.getElementsByClassName("badge");
        if (badges.length > 0) {
            counterBadge = badges[0];
        } else {
            counterBadge = document.createElement("span");
            counterBadge.classList.add("badge", "rounded-pill", "text-bg-danger");
            reviewAreaButton.appendChild(counterBadge);
        }

        counterBadge.innerText = count.toString();
        if (count === 0) {
            counterBadge.classList.add("d-none");
        } else {
            counterBadge.classList.remove("d-none");
        }
        if (animate && count > previousCount) {
            counterBadge.animate({"transform": ["scale(1)", "scale(2)"]}, {
                duration: 250,
                iterations: 2,
                direction: "alternate",
                fill: "both",
                easing: "ease-in"
            });
        }
        previousCount = count;
    }

    init();
}
