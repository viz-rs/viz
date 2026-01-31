use bevy_app::SubApp;
use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::{
    query::QueryState,
    world::{Mut, World},
};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{
        Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel, RenderSubGraph,
    },
    render_phase::ViewSortedRenderPhases,
    render_resource::RenderPassDescriptor,
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

use crate::render::{
    transparent::TransparentUi,
    view::{MoonUiCameraView, MoonUiViewTarget},
};

/// Moon ui subgraph (is run by [`super::RunMoonUiSubgraphOnMoonUiViewNode`]).
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
pub struct SubGraphMoonUi;

/// Moon ui node defining the moon ui rendering pass.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub enum NodeMoonUi {
    /// Moon ui rendering pass.
    MoonUiPass,
}

/// Moon ui pass node.
pub struct MoonUiPassNode {
    view_query: QueryState<(&'static ExtractedView, &'static MoonUiViewTarget)>,
    view_target_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        Option<&'static MainPassResolutionOverride>,
    )>,
}

impl MoonUiPassNode {
    /// Creates an moon ui pass node.
    pub fn new(world: &mut World) -> Self {
        Self {
            view_query: world.query_filtered(),
            view_target_query: world.query(),
        }
    }
}

impl Node for MoonUiPassNode {
    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
        self.view_target_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Extract the UI view.
        let input_view_entity = graph.view_entity();

        // Query the UI view components.
        let Ok((view, view_target)) = self.view_query.get_manual(world, input_view_entity) else {
            return Ok(());
        };

        let Ok((camera, target, resolution_override)) =
            self.view_target_query.get_manual(world, view_target.0)
        else {
            return Ok(());
        };

        let Some(render_phases) = world.get_resource::<ViewSortedRenderPhases<TransparentUi>>()
        else {
            return Ok(());
        };

        let Some(render_phase) = render_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let color_attachment = target.get_color_attachment(); // sample count 4
        // let color_attachment = target.get_unsampled_color_attachment(); // sample count 1
        let depth_stencil_attachment = None;

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("moon ui"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let pass_span = diagnostics.pass_span(&mut render_pass, "moon ui");

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        if let Err(err) = render_phase.render(&mut render_pass, world, input_view_entity) {
            tracing::error!("Error encountered while rendering the stencil phase {err:?}");
        }

        pass_span.end(&mut render_pass);

        Ok(())
    }
}

/// A [`Node`] that executes the moon ui rendering subgraph on the moon ui view.
pub struct RunMoonUiSubgraphOnMoonUiViewNode;

impl Node for RunMoonUiSubgraphOnMoonUiViewNode {
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        _: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Fetch the UI view.
        let Some(mut render_views) = world.try_query::<&MoonUiCameraView>() else {
            return Ok(());
        };
        let Ok(default_camera_view) = render_views.get(world, graph.view_entity()) else {
            return Ok(());
        };

        // Run the subgraph on the moon ui view.
        graph.run_sub_graph(SubGraphMoonUi, vec![], Some(default_camera_view.0), None)?;
        Ok(())
    }
}

// Adds moon ui subgraph to the 2D/3D graph.
pub fn add_moon_ui_subgraph(render_app: &mut SubApp) {
    let world = render_app.world_mut();
    add_moon_ui_subgraph_to_2d(world);
    add_moon_ui_subgraph_to_3d(world);
}

/// Adds and returns an moon ui subgraph.
fn get_moon_ui_subgraph(world: &mut World) -> RenderGraph {
    let pass_node = MoonUiPassNode::new(world);
    let mut graph = RenderGraph::default();
    graph.add_node(NodeMoonUi::MoonUiPass, pass_node);
    graph
}

// Adds moon ui subgraph to the 2D graph.
fn add_moon_ui_subgraph_to_2d(world: &mut World) {
    use bevy_core_pipeline::core_2d::graph::{Core2d, Node2d};

    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        let Some(graph_2d) = graph.get_sub_graph_mut(Core2d) else {
            return;
        };

        let moon_ui_graph = get_moon_ui_subgraph(world);
        graph_2d.add_sub_graph(SubGraphMoonUi, moon_ui_graph);
        graph_2d.add_node(NodeMoonUi::MoonUiPass, RunMoonUiSubgraphOnMoonUiViewNode);
        graph_2d.add_node_edge(Node2d::EndMainPass, NodeMoonUi::MoonUiPass);
        graph_2d.add_node_edge(Node2d::EndMainPassPostProcessing, NodeMoonUi::MoonUiPass);
        graph_2d.add_node_edge(NodeMoonUi::MoonUiPass, Node2d::Upscaling);
    });
}

// Adds moon ui subgraph to the 3D graph.
fn add_moon_ui_subgraph_to_3d(world: &mut World) {
    use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};

    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        let Some(graph_3d) = graph.get_sub_graph_mut(Core3d) else {
            return;
        };

        let moon_ui_graph = get_moon_ui_subgraph(world);
        graph_3d.add_sub_graph(SubGraphMoonUi, moon_ui_graph);
        graph_3d.add_node(NodeMoonUi::MoonUiPass, RunMoonUiSubgraphOnMoonUiViewNode);
        graph_3d.add_node_edge(Node3d::EndMainPass, NodeMoonUi::MoonUiPass);
        graph_3d.add_node_edge(Node3d::EndMainPassPostProcessing, NodeMoonUi::MoonUiPass);
        graph_3d.add_node_edge(NodeMoonUi::MoonUiPass, Node3d::Upscaling);
    });
}
