//! Cached storage inventory; filesystem work never runs on the render thread.
use super::*;
use crate::glass_ui as ui;
use horizon_launcher::browser;
use storage::Category;

impl Launcher {
    pub(super) fn refresh_storage(&mut self, clear: Option<Category>, all: bool) {
        if self.storage_pending {
            return;
        }
        self.storage_pending = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = storage::Storage::open()
                .and_then(|storage| storage.report(clear, all))
                .map(Box::new)
                .map_err(native_mods::cache_error);
            let _ = tx.send(Msg::Storage(result));
        });
    }

    pub(super) fn glass_storage(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let blocked = self.storage_pending
            || self.storage_error.as_deref()
                == Some(tr(
                    "A native build or cache cleanup is running. Try again when it finishes.",
                ));
        let mut card = crate::glass::plate(horizon_theme::glass::radius::PANEL)
            .flex()
            .flex_col()
            .p(px(16.))
            .gap(px(12.));
        let total = self.storage_report.as_ref().map_or_else(
            || tr("Calculating…").to_string(),
            |report| storage::format_bytes(report.rows.iter().map(|row| row.remaining_bytes).sum()),
        );
        card = card.child(ui::setting_row(
            tr("Storage"),
            Some(div().child(total).into_any_element()),
            Some(
                button("storage-clear-all", "Clear all", Kind::Secondary, &t)
                    .opacity(if blocked { 0.4 } else { 1.0 })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !blocked {
                            this.refresh_storage(None, true);
                        }
                        cx.notify();
                    }))
                    .into_any_element(),
            ),
        ));
        let limit = segments(
            "storage-limit",
            &[
                (10u64, "10 GB"),
                (25, "25 GB"),
                (50, "50 GB"),
                (100, "100 GB"),
                (0, "No limit"),
            ],
            self.settings.cache_limit_gib.unwrap_or(25),
            &t,
            {
                let this = cx.entity().downgrade();
                move |value, _, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.settings.cache_limit_gib = Some(value);
                        if let Err(error) = this.settings.save() {
                            this.storage_error = Some(error);
                        }
                        cx.notify();
                    });
                }
            },
        );
        card = card.child(ui::setting_row(
            tr("Cache limit"),
            None,
            Some(limit.into_any_element()),
        ));
        if self.client.is_some() {
            card = card.child(tr("A game is running. Files it may use are kept."));
        }
        if self.storage_pending {
            card = card.child(tr("A build or storage operation is running."));
        }
        card = card.child(
            button("storage-refresh", "Refresh", Kind::Secondary, &t).on_click(cx.listener(
                |this, _, _, cx| {
                    this.refresh_storage(None, false);
                    cx.notify();
                },
            )),
        );
        if let Some(error) = &self.storage_error {
            card = card.child(error.clone());
        }
        let summary = card;
        let mut card = crate::glass::plate(horizon_theme::glass::radius::PANEL)
            .flex()
            .flex_col()
            .p(px(8.));
        for (category, label) in Category::ALL.into_iter().zip([
            "Build files",
            "Prebuilt games",
            "Spare game builds",
            "Rust compiler",
            "Other caches",
        ]) {
            let row = self
                .storage_report
                .as_ref()
                .and_then(|report| report.rows.iter().find(|row| row.category == category));
            let mut detail = div().child(row.map_or_else(
                || tr("Calculating…").to_string(),
                |row| storage::format_bytes(row.remaining_bytes),
            ));
            if let Some(row) = row {
                for kept in &row.kept {
                    detail = detail.child(div().child(trf(
                        "Space it couldn't free: {bytes}. {reason}",
                        &[
                            ("bytes", &storage::format_bytes(kept.bytes)),
                            ("reason", &kept.reason),
                        ],
                    )));
                }
            }
            card = card.child(ui::setting_row(
                tr(label),
                Some(detail.into_any_element()),
                Some(
                    button(
                        SharedString::from(format!("storage-clear-{category:?}")),
                        "Clear",
                        Kind::Secondary,
                        &t,
                    )
                    .opacity(if blocked { 0.4 } else { 1.0 })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !blocked {
                            this.refresh_storage(Some(category), false);
                        }
                        cx.notify();
                    }))
                    .into_any_element(),
                ),
            ));
        }
        let breakdown = card;
        let mut card = crate::glass::plate(horizon_theme::glass::radius::PANEL).p(px(8.));
        if self.settings.native_mods_allowed_at.is_some() {
            card = card.child(ui::setting_row(
                tr("Native mods"),
                Some(
                    div()
                        .child(tr("Native mods have access to your computer."))
                        .into_any_element(),
                ),
                Some(
                    button("storage-revoke", "Turn off", Kind::Secondary, &t)
                        .on_click(cx.listener(|this, _, _, cx| {
                            let result = this.settings.revoke_native_mods().and_then(|()| {
                                let mut failure = None;
                                for instance in instances::list() {
                                    for (id, _, _) in native_status::enabled(&instance.home) {
                                        if let Err(error) =
                                            browser::java::set_enabled(&instance.home, &id, false)
                                        {
                                            failure.get_or_insert(error);
                                        }
                                    }
                                }
                                failure.map_or(Ok(()), Err)
                            });
                            if let Err(error) = result {
                                this.storage_error = Some(error);
                            }
                            cx.notify();
                        }))
                        .into_any_element(),
                ),
            ));
        } else {
            card = card.child(tr("Native mods are off."));
        }
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(summary)
            .child(breakdown)
            .child(card)
    }
}
