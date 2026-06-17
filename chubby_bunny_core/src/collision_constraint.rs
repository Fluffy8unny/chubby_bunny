use crate::{Body, FloatingPointNumber, SolverSettings};
use itertools::Itertools;
use nalgebra::Vector2;

pub struct CollisionConstraint<T> {
    stiffness: T,
}

impl<T: Clone> Clone for CollisionConstraint<T> {
    fn clone(&self) -> Self {
        Self {
            stiffness: self.stiffness.clone(),
        }
    }
}

impl<T> CollisionConstraint<T> {
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
}
impl<T: FloatingPointNumber> Edge<T> {
    pub fn center(&self) -> Vector2<T> {
        (self.pt_a + self.pt_b) / T::from(2.0)
    }
}

fn body_to_edge_list<T: FloatingPointNumber>(body: &Body<T>) -> Vec<Edge<T>> {
    body.particles
        .iter()
        .enumerate()
        .circular_tuple_windows()
        .map(|((idx_a, _), (idx_b, _))| Edge {
            idx_a,
            idx_b,
            pt_a: body.particles[idx_a].position,
            pt_b: body.particles[idx_b].position,
        })
        .collect()
}
struct Intersection<T> {
    normal: Vector2<T>,
    rel_line_position_a: T,
    rel_line_position_b: T,
    penetration_depth: T,
}

#[derive(Clone)]
struct Contermination<T> {
    contained_point_idx: usize,
    edge: Edge<T>,
    rel_edge_position: T,
    normal: Vector2<T>,
    penetration_depth: T,
}

fn collision_epsilon<T: FloatingPointNumber>() -> T {
    T::from(1.0e-6_f32)
}

fn min_penetration_depth<T: FloatingPointNumber>() -> T {
    T::from(1.0e-5_f32)
}

fn point_segment_distance_squared<T: FloatingPointNumber>(
    point: Vector2<T>,
    segment_a: Vector2<T>,
    segment_b: Vector2<T>,
) -> (T, T, Vector2<T>) {
    let segment = segment_b - segment_a;
    let segment_len2 = segment.dot(&segment);
    if segment_len2 <= T::zero() {
        let diff = point - segment_a;
        return (diff.norm_squared(), T::zero(), segment_a);
    }

    let t = ((point - segment_a).dot(&segment) / segment_len2).clamp(T::zero(), T::one());
    let projection = segment_a + segment * t;
    let diff = point - projection;
    (diff.norm_squared(), t, projection)
}

fn edge_outward_normal<T: FloatingPointNumber>(edge: &Edge<T>) -> Option<Vector2<T>> {
    let edge_vector = edge.pt_b - edge.pt_a;
    if edge_vector.norm_squared() <= collision_epsilon::<T>() {
        return None;
    }

    // All polygons are defined CCW, so the outward normal is the negative left normal.
    let left_normal = Vector2::new(-edge_vector.y, edge_vector.x).normalize();
    Some(-left_normal)
}

fn find_containment_contacts<T: FloatingPointNumber>(
    contained_body: &Body<T>,
    container_body: &Body<T>,
) -> Vec<Contermination<T>> {
    let container_edges = body_to_edge_list(container_body);
    let mut contacts = Vec::new();

    let eps = collision_epsilon::<T>();

    for (contained_idx, contained_particle) in contained_body.particles.iter().enumerate() {
        let point = contained_particle.position;
        if !container_body.point_in_polygon(point) {
            continue;
        }

        let best = container_edges
            .iter()
            .map(|edge| {
                let (distance_sq, t, projection) =
                    point_segment_distance_squared(point, edge.pt_a, edge.pt_b);
                (edge, distance_sq, t, projection)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let Some((best_edge, best_distance_sq, best_t, best_projection)) = best else {
            continue;
        };

        let point_to_projection = best_projection - point;
        let Some(mut normal) = edge_outward_normal(best_edge) else {
            continue;
        };

        // Keep normals oriented toward the nearest boundary point to avoid inward pushes in concave regions.
        if point_to_projection.norm_squared() > eps * eps
            && point_to_projection.dot(&normal) < T::zero()
        {
            normal = -normal;
        };

        // Add a small slop so points cross the boundary instead of settling exactly on it.
        let penetration_depth = best_distance_sq.sqrt() + min_penetration_depth::<T>();
        contacts.push(Contermination {
            contained_point_idx: contained_idx,
            edge: best_edge.clone(),
            rel_edge_position: best_t,
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

fn get_line_segment_intersection<T: FloatingPointNumber>(
    edge_a: &Edge<T>,
    edge_b: &Edge<T>,
) -> Option<Intersection<T>> {
    let p = edge_a.pt_a;
    let r = edge_a.pt_b - edge_a.pt_a;
    let q = edge_b.pt_a;
    let s = edge_b.pt_b - edge_b.pt_a;

    let eps = collision_epsilon::<T>();
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

    let normal_a = Vector2::new(-r.y, r.x).normalize();
    let normal_b = Vector2::new(-s.y, s.x).normalize();

    let penetration_depth_a = overlap_depth_on_axis(edge_a, edge_b, &normal_a);
    let penetration_depth_b = overlap_depth_on_axis(edge_a, edge_b, &normal_b);

    let (mut normal, penetration_depth) = if penetration_depth_a < penetration_depth_b {
        (normal_a, penetration_depth_a)
    } else {
        (normal_b, penetration_depth_b)
    };

    let penetration_depth = penetration_depth.max(min_penetration_depth::<T>());

    let centroid_diff = edge_a.center() - edge_b.center();
    if centroid_diff.dot(&normal) < T::zero() {
        normal = -normal;
    }

    Some(Intersection {
        normal,
        rel_line_position_a: t,
        rel_line_position_b: u,
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
        let weight_a = T::one() - point_weight;
        let weight_b = point_weight;

        body.particles[edge.idx_a]
            .apply_position_correction_to_particle(&(correction_vector * weight_a));
        body.particles[edge.idx_b]
            .apply_position_correction_to_particle(&(correction_vector * weight_b));
    }

    fn resolve_containment_contacts(
        &self,
        contained_body: &mut Body<T>,
        container_body: &mut Body<T>,
        contacts: Vec<Contermination<T>>,
        time_correction_factor: T,
    ) {
        //tood replace 0.5 with weight based
        let push_factor = self.stiffness * time_correction_factor;

        for contact in contacts {
            let correction_vector = contact.normal * push_factor * contact.penetration_depth;
            contained_body.particles[contact.contained_point_idx]
                .apply_position_correction_to_particle(&(correction_vector));
            self.apply_position_correction_to_edge(
                container_body,
                &contact.edge,
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
        for (edge_a, edge_b) in body_to_edge_list(body_a)
            .iter()
            .cartesian_product(body_to_edge_list(body_b).iter())
        {
            if let Some(intersection) = get_line_segment_intersection(edge_a, edge_b) {
                let correction_vector = intersection.normal
                    * self.stiffness
                    * time_correction_factor
                    * intersection.penetration_depth;
                self.apply_position_correction_to_edge(
                    body_a,
                    edge_a,
                    &correction_vector,
                    intersection.rel_line_position_a,
                );
                self.apply_position_correction_to_edge(
                    body_b,
                    edge_b,
                    &(-correction_vector),
                    intersection.rel_line_position_b,
                );
            }
        }

        let contacts_a_in_b = find_containment_contacts(body_a, body_b);
        self.resolve_containment_contacts(body_a, body_b, contacts_a_in_b, time_correction_factor);

        let contacts_b_in_a = find_containment_contacts(body_b, body_a);
        self.resolve_containment_contacts(body_b, body_a, contacts_b_in_a, time_correction_factor);
    }
}
