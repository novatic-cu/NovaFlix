//! Biblioteca: favoritos, listas y «Para ver».

use egui::{vec2, Align, Layout, RichText, ScrollArea, Ui};
use novaflix_core::{catalog, db, Level, ListKind, TargetKind};

use crate::{
    app::{LibTab, NovaApp, Screen},
    card::Card,
    theme,
    widgets::{self, Kind as Btn},
};

impl NovaApp {
    pub(crate) fn screen_library(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
        ui.label(
            RichText::new("Biblioteca")
                .font(theme::title1())
                .color(p.text),
        );
        ui.horizontal(|ui| {
            for (tab, label) in [
                (LibTab::Favorites, "Favoritos"),
                (LibTab::Lists, "Listas"),
                (LibTab::Watch, "Para ver"),
            ] {
                if widgets::chip(ui, &p, label, self.lib_tab == tab).clicked() {
                    self.lib_tab = tab;
                }
            }
        });
        ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match self.lib_tab {
                LibTab::Favorites => self.library_favorites(ui),
                LibTab::Lists => self.library_lists(ui),
                LibTab::Watch => self.library_watch(ui),
            });
    }

    fn library_empty(&mut self, ui: &mut Ui, icon: &str, title: &str, text: &str) {
        let p = self.p;
        widgets::glass_frame(&p, theme::radius::LG).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.label(RichText::new(icon).size(40.0).color(p.text_3));
                ui.label(RichText::new(title).font(theme::title2()));
                ui.label(RichText::new(text).color(p.text_2));
                ui.add_space(8.0);
                if widgets::button(
                    ui,
                    &p,
                    Some(egui_phosphor::regular::MAGNIFYING_GLASS),
                    "Buscar algo",
                    Btn::Primary,
                )
                .clicked()
                {
                    self.go(Screen::Search);
                }
                ui.add_space(24.0);
            });
        });
    }

    fn library_favorites(&mut self, ui: &mut Ui) {
        let p = self.p;
        if self.favorites.is_empty() {
            self.library_empty(
                ui,
                egui_phosphor::regular::STAR,
                "Todavía no tienes favoritos",
                "Pulsa ☆ en cualquier título para guardarlo aquí.",
            );
            return;
        }
        let favorites = self.favorites.clone();
        let (folders, items): (Vec<_>, Vec<_>) =
            favorites.iter().partition(|f| f.kind == TargetKind::Folder);
        if !folders.is_empty() {
            widgets::section_header(ui, &p, "Carpetas", None);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
                for fav in &folders {
                    let card = Card::from_favorite(fav);
                    if self.draw_card(ui, &card, NovaApp::card_size()).clicked() {
                        self.open_sheet(&card.path, &card.name, true, None);
                    }
                }
            });
        }
        if !items.is_empty() {
            widgets::section_header(ui, &p, "Archivos", None);
            for fav in &items {
                let card = Card::from_favorite(fav);
                self.file_row(ui, &card);
            }
        }
    }

    fn library_lists(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_list_name)
                    .hint_text("Nombre de la nueva lista")
                    .desired_width(260.0),
            );
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::PLUS),
                "Crear lista",
                Btn::Primary,
            )
            .clicked()
            {
                match self
                    .core
                    .list_create(&self.new_list_name, ListKind::Playlist)
                {
                    Ok(_) => {
                        self.new_list_name.clear();
                        self.refresh_library();
                    }
                    Err(err) => self.toast(Level::Warn, err.to_string()),
                }
            }
        });
        let lists: Vec<_> = self
            .lists
            .iter()
            .filter(|l| l.kind == ListKind::Playlist)
            .cloned()
            .collect();
        if lists.is_empty() {
            self.library_empty(
                ui,
                egui_phosphor::regular::LIST,
                "Crea tu primera lista",
                "Agrupa títulos para verlos después o exportarlos a VLC.",
            );
            return;
        }
        for list in lists {
            widgets::glass_frame(&p, theme::radius::MD).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&list.name).font(theme::headline()));
                    ui.label(
                        RichText::new(format!("{} elementos", list.count))
                            .font(theme::caption())
                            .color(p.text_3),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(
                            ui,
                            &p,
                            egui_phosphor::regular::TRASH,
                            "Eliminar lista",
                        )
                        .clicked()
                        {
                            let _ = self.core.list_delete(list.id);
                            self.refresh_library();
                        }
                        if widgets::button(
                            ui,
                            &p,
                            Some(egui_phosphor::regular::EXPORT),
                            "Exportar M3U",
                            Btn::Secondary,
                        )
                        .clicked()
                        {
                            match self.core.export_m3u(list.id) {
                                Ok(file) => self
                                    .toast(Level::Info, format!("Exportada en {}", file.display())),
                                Err(err) => self.toast(Level::Error, err.to_string()),
                            }
                        }
                    });
                });
                self.list_items_ui(ui, list.id);
            });
        }
    }

    fn list_items_ui(&mut self, ui: &mut Ui, id: i64) {
        let p = self.p;
        let items = self.core.list_items(id).unwrap_or_default();
        let paths: Vec<String> = items.iter().map(|i| i.path.clone()).collect();
        for (index, item) in items.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}", index + 1))
                        .font(theme::caption())
                        .color(p.text_3),
                );
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(catalog::decode_name(&item.name)).font(theme::body()),
                        )
                        .frame(false),
                    )
                    .clicked()
                {
                    let name = catalog::decode_name(&db::name_of(&item.path));
                    self.open_sheet(&item.path, &name, item.path.ends_with('/'), None);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::icon_button(ui, &p, egui_phosphor::regular::X, "Quitar de la lista")
                        .clicked()
                    {
                        let _ = self.core.list_remove(id, &item.path);
                        self.refresh_library();
                    }
                    if index + 1 < paths.len()
                        && widgets::icon_button(ui, &p, egui_phosphor::regular::ARROW_DOWN, "Bajar")
                            .clicked()
                    {
                        let mut order = paths.clone();
                        order.swap(index, index + 1);
                        let _ = self.core.list_reorder(id, order);
                    }
                    if index > 0
                        && widgets::icon_button(ui, &p, egui_phosphor::regular::ARROW_UP, "Subir")
                            .clicked()
                    {
                        let mut order = paths.clone();
                        order.swap(index, index - 1);
                        let _ = self.core.list_reorder(id, order);
                    }
                });
            });
        }
    }

    fn library_watch(&mut self, ui: &mut Ui) {
        let p = self.p;
        let Some(list) = self
            .lists
            .iter()
            .find(|l| l.kind == ListKind::Watchlist)
            .cloned()
        else {
            self.library_empty(
                ui,
                egui_phosphor::regular::EYE,
                "Nada pendiente",
                "Usa «Lista → Para ver» en la ficha de un título.",
            );
            return;
        };
        let items = self.core.list_items(list.id).unwrap_or_default();
        if items.is_empty() {
            self.library_empty(
                ui,
                egui_phosphor::regular::EYE,
                "Nada pendiente",
                "Usa «Lista → Para ver» en la ficha de un título.",
            );
            return;
        }
        for item in items {
            widgets::glass_frame(&p, theme::radius::MD).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let mut watched = item.watched_at.is_some();
                    if ui.checkbox(&mut watched, "").changed() {
                        let _ = self.core.list_mark_watched(list.id, &item.path, watched);
                    }
                    let title = RichText::new(crate::format::clean_title(&catalog::decode_name(
                        &item.name,
                    )))
                    .font(theme::headline());
                    let title = if item.watched_at.is_some() {
                        title.color(p.text_3).strikethrough()
                    } else {
                        title.color(p.text)
                    };
                    if ui.add(egui::Button::new(title).frame(false)).clicked() {
                        let name = catalog::decode_name(&db::name_of(&item.path));
                        self.open_sheet(&item.path, &name, item.path.ends_with('/'), None);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::icon_button(ui, &p, egui_phosphor::regular::X, "Quitar")
                            .clicked()
                        {
                            let _ = self.core.list_remove(list.id, &item.path);
                            self.refresh_library();
                        }
                    });
                });
            });
        }
    }
}
