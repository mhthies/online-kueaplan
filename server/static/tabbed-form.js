function initializeTabbedForm(tablist) {
    const tabs = Array.from(tablist.querySelectorAll('[data-bs-toggle="tab"]'));
    for (const tab of tabs) {
        bootstrap.Tab.getOrCreateInstance(tab);
    }

    window.addEventListener("popstate", (event) => {
        if (!event.state || event.state.tab === undefined) {
            // Not a history state created by tabbed-form.
            return;
        }
        const tab = tabs.find((tab) => tab.id === event.state.tab);
        if (!tab) {
            console.warn(`Could not find tab ${event.state.tab} form history state.`);
            return;
        }
        tab.isHistoryPop = true;
        bootstrap.Tab.getInstance(tab).show();
    });

    function findTabPane(tab) {
        const tabPaneId = tab.getAttribute("data-bs-target");
        if (!tabPaneId) {
            console.error("Could not find tab pane of tab: Missing data-bs-target attribute", tab);
            return undefined;
        }
        const tabPane = document.querySelector(tabPaneId);
        if (!tabPane) {
            console.error("Could not find tab pane of tab: Query selector did not match any element", tab);
            return undefined;
        }
        return tabPane;
    }

    function createButtonEventListeners(tabPane, tabIndex) {
        const nextButtons = tabPane.querySelectorAll('[data-tab-button="next"]');
        for (const button of nextButtons) {
            if (tabIndex + 1 >= tabs.length) {
                console.error("Cannot add click handler for next-button. Is in last tab.", button);
                continue;
            }
            button.addEventListener('click', (e) => {
                bootstrap.Tab.getInstance(tabs[tabIndex + 1]).show();
            });
        }
        const prevButtons = tabPane.querySelectorAll('[data-tab-button="prev"]');
        for (const button of prevButtons) {
            if (tabIndex - 1 < 0) {
                console.error("Cannot add click handler for previous-button. Is in first tab.", button);
                continue;
            }
            button.addEventListener('click', (e) => {
                bootstrap.Tab.getInstance(tabs[tabIndex - 1]).show();
            });
        }
    }

    function createTabbingEventListeners(tab, tabPane) {
        const submitButtons = tabPane.querySelectorAll('button[type="submit"]');
        tab.addEventListener("hidden.bs.tab", (event) => {
            submitButtons.forEach((button) => {
                button.disabled = true;
            });
        });
        tab.addEventListener("show.bs.tab", (event) => {
            const tab = event.target;
            if (!tab.isHistoryPop) {
                history.pushState({"tab": tab.id}, "", "#" + tab.id);
            }
            tab.isHistoryPop = false;
            submitButtons.forEach((button) => {
                button.disabled = false;
            });
        });
    }

    function createErrorIndicator(tab) {
        tab.innerHTML +=
            '&ensp;<span class="text-danger">' +
            '<i class="bi bi-exclamation-octagon" aria-hidden="true"></i>' +
            '<span class="visually-hidden">(enthält Validierungsfehler)</span>' +
            '</span>';
    }

    function determineInitialTab() {
        let tabFromUrl = null;
        if (window.location.hash) {
            const tabId = window.location.hash.substring(1);
            tabFromUrl = tabs.find((tab) => tab.id === tabId);
            if (!tabFromUrl) {
                console.info(`'${tabId}', the URL anchor, is not an id of one of the tabs. Ignoring it.`);
            }
        }
        if (tabFromUrl) {
            return tabFromUrl
        }
        const firstTabWithErrors = tabs.find((tab) => tab.hasValidationErrors);
        return firstTabWithErrors;
    }

    function activateInitialTab() {
        let tab = determineInitialTab();
        if (tab) {
            tab.isHistoryPop = true;
            bootstrap.Tab.getInstance(tab).show();
        } else {
            tab = tabs.find((tab) => tab.classList.contains("active"));
        }
        if (tab) {
            history.replaceState({"tab": tab.id}, "", "#" + tab.id);
        }
        return tab;
    }

    function disableSubmitButtons(tabPane) {
        const submitButtons = tabPane.querySelectorAll('button[type="submit"]');
        submitButtons.forEach((button) => {
            button.disabled = true;
        });
    }

    function init() {
        const tabPanes = tabs.map(findTabPane);
        for (const [tabIndex, tab, tabPane] of tabs.map((e, i) => [i, e, tabPanes[i]])) {
            createButtonEventListeners(tabPane, tabIndex);
            createTabbingEventListeners(tab, tabPane);
            const hasValidationErrors = tabPane.getElementsByClassName('is-invalid').length > 0;
            if (hasValidationErrors) {
                createErrorIndicator(tab);
                tab.hasValidationErrors = true;
            }
        }

        const activeTab = activateInitialTab();

        for (const [tab, tabPane] of tabs.map((e, i) => [e, tabPanes[i]])) {
            if (tab !== activeTab) {
                disableSubmitButtons(tabPane);
            }
        }
    }

    init();
}
