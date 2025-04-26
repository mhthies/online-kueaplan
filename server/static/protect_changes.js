/**
 * Register a beforeunload event handler to protect the user from accidentally leaving the page with unsaved changes in
 * the given HTML form.
 *
 * The confirmation dialog is only invoked when the form has been changed compared to the initial state or the
 * `data-consider-changed` attribute is set on the form. The confirmation dialog is not invoked when the form is
 * submitted or a cancel button is clicked.
 */
function protectChanges(form) {
    // https://stackoverflow.com/a/52593167
    function initChangeDetection(form) {
        let serializedFormData = new URLSearchParams(new FormData(form)).toString();
        form.originalState = serializedFormData;
    }
    function formHasChanges(form) {
        let serializedFormData = new URLSearchParams(new FormData(form)).toString();
        return form.originalState !== undefined && form.originalState !== serializedFormData;
    }

    initChangeDetection(form);
    form.addEventListener('submit', () => {
        // Don't ask for confirmation when the form is being submitted
        form.allowDirtyExitWithoutConfirm = true;
    });
    for (let button of form.getElementsByClassName("allow-exit-with-changes")) {
        // Don't ask for confirmation when an 'abort' button in the form is clicked
        button.addEventListener('click', () => {
            form.allowDirtyExitWithoutConfirm = true;
        });
    }
    for (let button of document.getElementsByClassName("allow-exit-with-changes-global")) {
        // Don't ask for confirmation when a global 'abort' button is clicked
        button.addEventListener('click', () => {
            form.allowDirtyExitWithoutConfirm = true;
        });
    }

    // See https://developer.mozilla.org/en-US/docs/Web/API/Window/beforeunload_event
    window.addEventListener('beforeunload', (e) => {
        if ((form.getAttribute("data-consider-changed") || formHasChanges(form))
            && !form.allowDirtyExitWithoutConfirm)
        {
            e.preventDefault();
        }
    });
}
