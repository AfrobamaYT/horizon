use super::*;

impl Game<'_> {
    /// A stack's tooltip: its lines, and under them a bundle's contents
    /// (`ClientBundleTooltip`: the 4 wide grid, the `+N` of what it leaves
    /// out, and the weight bar).
    pub(super) fn draw_stack_tooltip(
        &mut self,
        batch: &mut HudBatch,
        stack: &ItemStack,
        x: f32,
        y: f32,
    ) {
        self.draw_stack_tooltip_with(batch, stack, false, x, y, Vec::new());
    }
    /// [`Self::draw_stack_tooltip`] with `TooltipFlag.isCreative` set when
    /// `creative` (a creative tab's own slot), and `after_name` inserted from
    /// line 1, as the creative screen adds its tab names.
    pub(super) fn draw_stack_tooltip_with(
        &mut self,
        batch: &mut HudBatch,
        stack: &ItemStack,
        creative: bool,
        x: f32,
        y: f32,
        after_name: Vec<(horizon_client::text::Text, u32)>,
    ) {
        let mut lines = self.stack_tooltip_lines(stack, false, creative);
        if lines.is_empty() {
            return;
        }
        lines.splice(1..1, after_name);
        // The component's height does not depend on where it is drawn, so a
        // probe at the origin gives the box its extra height.
        let probe =
            horizon_client::item_tooltip::bundle_tooltip(self.items(), stack, 0.0, 0.0, 96.0);
        let bundle_extra = probe.as_ref().map_or(0.0, |l| l.height + 2.0);
        let mut component = None;
        self.native_tooltip_component(stack, &mut component);
        let images: Vec<TooltipImage> = tooltip_component_size(component)
            .and_then(|size| {
                Some(TooltipImage {
                    stack: stack.try_into_api().ok()?,
                    size,
                })
            })
            .into_iter()
            .collect();
        let Some((lines, images)) =
            self.gather_tooltip_components(stack, lines, images, (batch.width, batch.height))
        else {
            return;
        };
        if lines.is_empty() && images.is_empty() {
            return;
        }
        let lines = Self::resolved(&lines);
        let image_h: f32 = images.iter().map(|image| image.size[1] + 2.0).sum();
        let extra = bundle_extra + image_h;
        let min_width = images
            .iter()
            .fold(if probe.is_some() { 96.0_f32 } else { 0.0 }, |w, image| {
                w.max(image.size[0])
            });
        let screen = (batch.width, batch.height);
        let Some(at) = self.tooltip_pre(batch, Some(stack), &lines, &images, (x, y), screen) else {
            return;
        };
        let atlas = &self.gui.atlas;
        let (x, y, w, h) = Self::tooltip_geometry(atlas, &lines, at, screen, [min_width, extra]);
        self.tooltip_box(batch, Some(stack), x, y, w, h);
        let text_h = h - extra;
        Self::tooltip_text(batch, atlas, &lines, x, y);
        if let Some(layout) = horizon_client::item_tooltip::bundle_tooltip(
            self.items(),
            stack,
            x,
            y + text_h + 2.0,
            w,
        ) {
            self.draw_bundle_tooltip(batch, &layout);
            self.draw_bundle_selected_name(batch, &layout, x, y + text_h + 2.0, w);
        }
        let mut at = y + text_h + bundle_extra + 2.0;
        for image in &images {
            if let Ok(stack) = crate::game::api_values::stack_value(&image.stack) {
                self.native_tooltip_component_draw(&stack, batch, [x, at], image.size);
            }
            at += image.size[1] + 2.0;
        }
    }
    /// `RenderTooltipEvent.GatherComponents` for an item's tooltip: the
    /// lines and the images the item's part answered go to
    /// `render_tooltip_gather_components` as one list. A mod's `max_width`
    /// wraps the lines (`Font.split`; unset, -1, leaves them as they are); an
    /// image a mod gave an invalid size is dropped, as an invalid answer of
    /// the part is. `None` when a mod cancelled it, so nothing is drawn.
    fn gather_tooltip_components(
        &self,
        stack: &ItemStack,
        lines: Vec<(horizon_client::text::Text, u32)>,
        images: Vec<TooltipImage>,
        (width, height): (f32, f32),
    ) -> Option<(Vec<(horizon_client::text::Text, u32)>, Vec<TooltipImage>)> {
        if !mod_api::__private::listened(mod_api::HookId::RenderTooltipGatherComponents) {
            return Some((lines, images));
        }
        let mut elements: Vec<TooltipElement> = lines
            .into_iter()
            .map(|(text, color)| {
                text.try_into_api()
                    .map(|text| TooltipElement::Text(text, color))
            })
            .collect::<Result<_, _>>()
            .ok()?;
        elements.extend(images.into_iter().map(TooltipElement::Image));
        let (mut max_width, mut cancel) = (-1, false);
        self.native_render_tooltip_gather_components(
            stack,
            width as i32,
            height as i32,
            &mut elements,
            &mut max_width,
            &mut cancel,
        );
        if cancel {
            return None;
        }
        let font = &self.gui.atlas.font;
        let (mut lines, mut images) = (Vec::new(), Vec::new());
        for element in elements {
            match element {
                TooltipElement::Text(text, color) => {
                    let text = text.try_into_engine(&()).ok()?;
                    if max_width < 0 {
                        lines.push((text, color));
                    } else {
                        lines.extend(
                            font.split(&text.resolve_legacy(), max_width as f32)
                                .into_iter()
                                .map(|line| (line, color)),
                        );
                    }
                }
                TooltipElement::Image(image) => {
                    if let Some(size) = tooltip_component_size(Some(image.size)) {
                        images.push(TooltipImage { size, ..image });
                    }
                }
            }
        }
        Some((lines, images))
    }
}
