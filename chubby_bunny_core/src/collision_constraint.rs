use crate::{
    constraint_common::{distribute_based_on_mass, get_normal},
    eps, Body, FloatingPointNumber, SolverSettings,
};
use itertools::Itertools;
use nalgebra::Vector2;

/// Collision between two bodies, that are the children of a given parent body, where both bodies can move and influence each other.
///
/// This is done in two steps:
/// First, edge intersections between the two bodies are detected and resolved by applying position corrections to the intersecting edges.
///  Second, it is checked if one body is completely or partially contained in the other body, which is resolved by applying position corrections
///  along the normal from the contained point to the nearest edge of the container body.
#[derive(Clone)]
pub struct CollisionConstraint<T> {
    /// Solver stiffness in `[0, 1]` where higher values enforce the target more strongly.
    stiffness: T,
}

impl<T> CollisionConstraint<T> {
    /// Creates a new collision constraint with the given stiffness.
    pub fn new(stiffness: T) -> Self {
        Self { stiffness }
    }
}

#[derive(Clone)]
struct Edge<T> {
    idx_a: usize,
    idx_b: usize,
    pt_a: Vector2<T>,
    pt_b: Vector2<T>,
    dir: Vector2<T>,
    center: Vector2<T>,
    normal: Option<Vector2<T>>,
    len_sq: T,
    min: Vector2<T>,
    max: Vector2<T>,
}
impl<T: FloatingPointNumber> Edge<T> {
    fn intersects(&self, other: &Edge<T>) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}
struct Intersection<T> {
    normal: Vector2<T>,
    edge_a_t: T,
    edge_b_t: T,
    penetration_depth: T,
}

#[derive(Clone)]
struct ContainmentContact<T> {
    contained_point_idx: usize,
    edge_idx: usize,
    rel_edge_position: T,
    normal: Vector2<T>,
    penetration_depth: T,
}

struct PointSegmentDistance<T> {
    distance_squared: T,
    t: T,
    projection: Vector2<T>,
}

fn edges_of<T: FloatingPointNumber>(body: &Body<T>) -> Vec<Edge<T>> {
    body.particles
        .iter()
        .enumerate()
        .circular_tuple_windows()
        .map(|((idx_a, _), (idx_b, _))| {
            let pt_a = body.particles[idx_a].position;
            let pt_b = body.particles[idx_b].position;
            let dir = pt_b - pt_a;
            let len_sq = dir.norm_squared();

            //save evewything we can to avoid recomputing it later
            Edge {
                idx_a,
                idx_b,
                pt_a,
                pt_b,
                dir,
                center: (pt_a + pt_b) / T::from(2.0),
                normal: get_normal(pt_b, pt_a),
                len_sq,
                min: pt_a.inf(&pt_b),
                max: pt_a.sup(&pt_b),
            }
        })
        .collect()
}

fn nearest_edge_to_point<T: FloatingPointNumber>(
    point: Vector2<T>,
    edges: &[Edge<T>],
) -> Option<(usize, PointSegmentDistance<T>)> {
    edges
        .iter()
        .enumerate()
        .map(|(idx, edge)| {
            (
                idx,
                point_segment_distance_squared(point, edge.pt_a, edge.pt_b),
            )
        })
        .min_by(|a, b| {
            a.1.distance_squared
                .partial_cmp(&b.1.distance_squared)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn point_segment_distance_squared<T: FloatingPointNumber>(
    point: Vector2<T>,
    segment_a: Vector2<T>,
    segment_b: Vector2<T>,
) -> PointSegmentDistance<T> {
    let segment = segment_b - segment_a;
    let segment_magnitude = segment.norm_squared();
    if segment_magnitude <= T::zero() {
        let diff = point - segment_a;
        return PointSegmentDistance {
            distance_squared: diff.norm_squared(),
            t: T::zero(),
            projection: segment_a,
        };
    }

    let t = ((point - segment_a).dot(&segment) / segment_magnitude).clamp(T::zero(), T::one());
    let projection = segment_a + segment * t;
    let diff = point - projection;
    PointSegmentDistance {
        distance_squared: diff.norm_squared(),
        t,
        projection,
    }
}

fn find_containment_contacts<T: FloatingPointNumber>(
    contained_body: &Body<T>,
    container_body: &Body<T>,
    container_edges: &[Edge<T>],
) -> Vec<ContainmentContact<T>> {
    let mut contacts = Vec::new();
    let eps = eps!(T, 6);

    for (contained_idx, contained_particle) in contained_body.particles.iter().enumerate() {
        let point = contained_particle.position;
        if !container_body.point_in_polygon(point) {
            continue;
        }

        let Some((best_edge_idx, nearest)) = nearest_edge_to_point(point, container_edges) else {
            continue;
        };
        let best_edge = &container_edges[best_edge_idx];

        if best_edge.len_sq <= eps {
            continue;
        }

        let Some(mut normal) = best_edge.normal else {
            continue;
        };

        let point_to_projection = nearest.projection - point;
        if point_to_projection.norm_squared() > eps * eps
            && point_to_projection.dot(&normal) < T::zero()
        {
            normal = -normal;
        }

        let penetration_depth = nearest.distance_squared.sqrt() + eps!(T, 5);
        contacts.push(ContainmentContact {
            contained_point_idx: contained_idx,
            edge_idx: best_edge_idx,
            rel_edge_position: nearest.t,
            normal,
            penetration_depth,
        });
    }
    contacts
}

fn overlap_depth_on_axis<T: FloatingPointNumber>(
    edge_a: &Edge<T>,
    edge_b: &Edge<T>,
    axis: &Vector2<T>,
) -> T {
    let a0 = edge_a.pt_a.dot(axis);
    let a1 = edge_a.pt_b.dot(axis);
    let b0 = edge_b.pt_a.dot(axis);
    let b1 = edge_b.pt_b.dot(axis);

    let a_min = a0.min(a1);
    let a_max = a0.max(a1);
    let b_min = b0.min(b1);
    let b_max = b0.max(b1);

    (a_max.min(b_max) - a_min.max(b_min)).max(T::zero())
}

fn segment_intersection<T: FloatingPointNumber>(
    edge_a: &Edge<T>,
    edge_b: &Edge<T>,
) -> Option<Intersection<T>> {
    if !edge_a.intersects(edge_b) {
        return None;
    }

    let p = edge_a.pt_a;
    let r = edge_a.dir;
    let q = edge_b.pt_a;
    let s = edge_b.dir;

    let eps = eps!(T, 6);
    let r_cross_s = r.perp(&s);
    if r_cross_s.abs() <= eps {
        return None; // Lines are parallel
    }

    let t_raw = (q - p).perp(&s) / r_cross_s;
    let u_raw = (q - p).perp(&r) / r_cross_s;

    //outside of segment
    if t_raw < -eps || t_raw > T::one() + eps || u_raw < -eps || u_raw > T::one() + eps {
        return None;
    }

    let t = t_raw.clamp(T::zero(), T::one());
    let u = u_raw.clamp(T::zero(), T::one());

    if edge_a.len_sq <= eps || edge_b.len_sq <= eps {
        return None;
    }

    let normal_a = edge_a.normal?;
    let normal_b = edge_b.normal?;

    let penetration_depth_a = overlap_depth_on_axis(edge_a, edge_b, &normal_a);
    let penetration_depth_b = overlap_depth_on_axis(edge_a, edge_b, &normal_b);

    let (mut normal, penetration_depth) = if penetration_depth_a < penetration_depth_b {
        (normal_a, penetration_depth_a)
    } else {
        (normal_b, penetration_depth_b)
    };

    let penetration_depth = penetration_depth.max(eps!(T, 5));

    let centroid_diff = edge_a.center - edge_b.center;
    if centroid_diff.dot(&normal) < T::zero() {
        normal = -normal;
    }

    Some(Intersection {
        normal,
        edge_a_t: t,
        edge_b_t: u,
        penetration_depth,
    })
}

impl<T: FloatingPointNumber> CollisionConstraint<T> {
    fn apply_position_correction_to_edge(
        &self,
        body: &mut Body<T>,
        edge: &Edge<T>,
        correction_vector: &Vector2<T>,
        point_weight: T,
    ) {
        let point_weight_a = T::one() - point_weight;
        let point_weight_b = point_weight;

        let (weight_a, weight_b) = distribute_based_on_mass(
            &body.particles[edge.idx_a],
            &body.particles[edge.idx_b],
            point_weight_a,
            point_weight_b,
        );

        body.particles[edge.idx_a]
            .apply_position_correction_to_particle(&(correction_vector * weight_a));
        body.particles[edge.idx_b]
            .apply_position_correction_to_particle(&(correction_vector * weight_b));
    }

    fn resolve_containment_contacts(
        &self,
        contained_body: &mut Body<T>,
        container_body: &mut Body<T>,
        container_edges: &[Edge<T>],
        contacts: Vec<ContainmentContact<T>>,
        time_correction_factor: T,
    ) {
        //tood replace 0.5 with weight based
        let correction_scale = self.stiffness * time_correction_factor;

        for contact in contacts {
            let correction_vector = contact.normal * correction_scale * contact.penetration_depth;
            contained_body.particles[contact.contained_point_idx]
                .apply_position_correction_to_particle(&(correction_vector));
            let edge = &container_edges[contact.edge_idx];
            self.apply_position_correction_to_edge(
                container_body,
                edge,
                &(-correction_vector),
                contact.rel_edge_position,
            );
        }
    }

    pub fn solve(
        &self,
        body_a: &mut Body<T>,
        body_b: &mut Body<T>,
        dt: T,
        solver_settings: &SolverSettings,
    ) {
        if !body_a
            .get_bounding_box()
            .intersects(&body_b.get_bounding_box())
        {
            return;
        }

        let time_correction_factor = dt
            / T::from(solver_settings.reference_dt * solver_settings.constraint_iterations as f32);
        let edges_a = edges_of(body_a);
        let edges_b = edges_of(body_b);

        for edge_a in &edges_a {
            for edge_b in &edges_b {
                let Some(intersection) = segment_intersection(edge_a, edge_b) else {
                    continue;
                };
                let correction_vector = intersection.normal
                    * self.stiffness
                    * time_correction_factor
                    * intersection.penetration_depth;
                self.apply_position_correction_to_edge(
                    body_a,
                    edge_a,
                    &correction_vector,
                    intersection.edge_a_t,
                );
                self.apply_position_correction_to_edge(
                    body_b,
                    edge_b,
                    &(-correction_vector),
                    intersection.edge_b_t,
                );
            }
        }

        let contacts_a_in_b = find_containment_contacts(body_a, body_b, &edges_b);
        self.resolve_containment_contacts(
            body_a,
            body_b,
            &edges_b,
            contacts_a_in_b,
            time_correction_factor,
        );

        let contacts_b_in_a = find_containment_contacts(body_b, body_a, &edges_a);
        self.resolve_containment_contacts(
            body_b,
            body_a,
            &edges_a,
            contacts_b_in_a,
            time_correction_factor,
        );
    }
}
