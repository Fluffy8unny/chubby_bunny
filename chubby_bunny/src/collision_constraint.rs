use crate::{Body, FloatingPointNumber, SolverSettings};
use itertools::Itertools;
use nalgebra::Vector2;

pub struct CollisionConstraint<T> {
    stiffness: T,
}

impl<T> CollisionConstraint<T> {
    pub fn new(stiffness: T) -> Self {
        Self { stiffness }
    }
}

struct BoundingBox<T> {
    min: Vector2<T>,
    max: Vector2<T>,
}

fn get_bounding_box<T: FloatingPointNumber>(body: &Body<T>) -> BoundingBox<T> {
    body.particles.iter().fold(
        BoundingBox {
            min: Vector2::new(T::max_value().unwrap(), T::max_value().unwrap()),
            max: Vector2::new(T::min_value().unwrap(), T::min_value().unwrap()),
        },
        |mut bbox, particle| {
            bbox.min.x = bbox.min.x.min(particle.position.x);
            bbox.min.y = bbox.min.y.min(particle.position.y);
            bbox.max.x = bbox.max.x.max(particle.position.x);
            bbox.max.y = bbox.max.y.max(particle.position.y);
            bbox
        },
    )
}

fn boxes_intersect<T: FloatingPointNumber>(box_a: &BoundingBox<T>, box_b: &BoundingBox<T>) -> bool {
    box_a.min.x <= box_b.max.x
        && box_a.max.x >= box_b.min.x
        && box_a.min.y <= box_b.max.y
        && box_a.max.y >= box_b.min.y
}

struct Edge<T> {
    idx_a: usize,
    idx_b: usize,
    pt_a: Vector2<T>,
    pt_b: Vector2<T>,
}
impl<T: FloatingPointNumber> Edge<T> {
    pub fn centroid(&self) -> Vector2<T> {
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

struct ContainmentContact<T> {
    contained_point_idx: usize,
    edge: Edge<T>,
    rel_edge_position: T,
    normal: Vector2<T>,
    penetration_depth: T,
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

fn point_in_polygon<T: FloatingPointNumber>(point: Vector2<T>, body: &Body<T>) -> bool {
    if body.particles.len() < 3 {
        return false;
    }

    let mut inside = false;
    for (a, b) in body.particles.iter().circular_tuple_windows() {
        let a = a.position;
        let b = b.position;

        let intersects = (a.y > point.y) != (b.y > point.y);
        if !intersects {
            continue;
        }

        let dy = b.y - a.y;
        if dy.abs() <= T::zero() {
            continue;
        }

        let x_intersection = a.x + (point.y - a.y) * (b.x - a.x) / dy;
        if point.x < x_intersection {
            inside = !inside;
        }
    }
    inside
}

fn edge_outward_normal<T: FloatingPointNumber>(
    edge: &Edge<T>,
    polygon_centroid: Vector2<T>,
) -> Vector2<T> {
    let edge_vector = edge.pt_b - edge.pt_a;
    let mut normal = Vector2::new(-edge_vector.y, edge_vector.x);
    let len2 = normal.norm_squared();
    if len2 <= T::zero() {
        return Vector2::zeros();
    }
    normal /= len2.sqrt();

    let edge_mid = (edge.pt_a + edge.pt_b) / T::from(2.0);
    let to_mid = edge_mid - polygon_centroid;
    if normal.dot(&to_mid) < T::zero() {
        normal = -normal;
    }
    normal
}

fn find_containment_contacts<T: FloatingPointNumber>(
    contained_body: &Body<T>,
    container_body: &Body<T>,
) -> Vec<ContainmentContact<T>> {
    let container_centroid = container_body.centroid();
    let container_edges = body_to_edge_list(container_body);
    let mut contacts = Vec::new();

    for (contained_idx, contained_particle) in contained_body.particles.iter().enumerate() {
        let point = contained_particle.position;
        if !point_in_polygon(point, container_body) {
            continue;
        }

        let mut best_distance_sq = T::max_value().unwrap();
        let mut best_edge: Option<&Edge<T>> = None;
        let mut best_t = T::zero();
        let mut best_projection = Vector2::zeros();

        for edge in &container_edges {
            let (distance_sq, t, projection) =
                point_segment_distance_squared(point, edge.pt_a, edge.pt_b);
            if distance_sq < best_distance_sq {
                best_distance_sq = distance_sq;
                best_edge = Some(edge);
                best_t = t;
                best_projection = projection;
            }
        }

        let Some(best_edge) = best_edge else {
            continue;
        };

        let mut normal = edge_outward_normal(best_edge, container_centroid);
        if normal.norm_squared() <= T::zero() {
            continue;
        }

        // Ensure the normal pushes the contained point away from the container.
        if (point - best_projection).dot(&normal) > T::zero() {
            normal = -normal;
        }

        let penetration_depth = best_distance_sq.sqrt();
        contacts.push(ContainmentContact {
            contained_point_idx: contained_idx,
            edge: Edge {
                idx_a: best_edge.idx_a,
                idx_b: best_edge.idx_b,
                pt_a: best_edge.pt_a.clone(),
                pt_b: best_edge.pt_b.clone(),
            },
            rel_edge_position: best_t,
            normal,
            penetration_depth,
        });
    }

    contacts
}

fn penetration_depth_along_normal<T: FloatingPointNumber>(
    edge_a: &Edge<T>,
    edge_b: &Edge<T>,
    normal: &Vector2<T>,
) -> T {
    let n_len2 = normal.norm_squared();
    if n_len2 <= T::zero() {
        return T::zero();
    }
    let n = *normal / n_len2.sqrt();

    let d0 = (edge_b.pt_a - edge_a.pt_a).dot(&n);
    let d1 = (edge_b.pt_b - edge_a.pt_a).dot(&n);

    d0.max(d1).max(T::zero())
}

fn get_line_segment_intersection<T: FloatingPointNumber>(
    edge_a: &Edge<T>,
    edge_b: &Edge<T>,
) -> Option<Intersection<T>> {
    let p = edge_a.pt_a;
    let r = edge_a.pt_b - edge_a.pt_a;
    let q = edge_b.pt_a;
    let s = edge_b.pt_b - edge_b.pt_a;

    let r_cross_s = r.perp(&s);
    if r_cross_s.abs() <= T::zero() {
        return None; // Lines are parallel
    }

    let t = (q - p).perp(&s) / r_cross_s;
    let u = (q - p).perp(&r) / r_cross_s;

    if t >= T::zero() && t <= T::one() && u >= T::zero() && u <= T::one() {
        let normal_a = Vector2::new(-r.y, r.x).normalize();
        let normal_b = Vector2::new(-s.y, s.x).normalize();

        let penetration_depth_a = penetration_depth_along_normal(edge_a, edge_b, &normal_a);
        let penetration_depth_b = penetration_depth_along_normal(edge_b, edge_a, &normal_b);

        let (mut normal, penetration_depth) = if penetration_depth_a < penetration_depth_b {
            (normal_a, penetration_depth_a)
        } else {
            (normal_b, penetration_depth_b)
        };

        let centroid_diff = edge_a.centroid() - edge_b.centroid();
        if centroid_diff.dot(&normal) < T::zero() {
            normal = -normal;
        }

        Some(Intersection {
            normal,
            rel_line_position_a: t,
            rel_line_position_b: u,
            penetration_depth,
        })
    } else {
        None
    }
}

impl<T: FloatingPointNumber> CollisionConstraint<T> {
    fn apply_position_correction_to_edge(
        &self,
        body: &mut Body<T>,
        edge: &Edge<T>,
        correction_vector: &Vector2<T>,
        point_weight: T,
    ) {
        let idx_a = edge.idx_a;
        let idx_b = edge.idx_b;
        let weight_a = T::one() - point_weight;
        let weight_b = point_weight;

        body.particles[idx_a].apply_position_correction(&(correction_vector * weight_a));
        body.particles[idx_b].apply_position_correction(&(correction_vector * weight_b));
    }

    fn resolve_containment_contacts(
        &self,
        contained_body: &mut Body<T>,
        container_body: &mut Body<T>,
        contacts: Vec<ContainmentContact<T>>,
        time_correction_factor: T,
    ) {
        let push_factor = self.stiffness * time_correction_factor;

        for contact in contacts {
            let correction_vector =
                contact.normal * push_factor * contact.penetration_depth * T::from(0.5);
            contained_body.particles[contact.contained_point_idx]
                .apply_position_correction(&(correction_vector));
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
        if !boxes_intersect(&get_bounding_box(body_a), &get_bounding_box(body_b)) {
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
