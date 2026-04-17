use rustc_middle::mir::{
    Location, Place, Rvalue,
    visit::{PlaceContext, Visitor},
};

pub fn rvalue_places<'tcx>(rvalue: &Rvalue<'tcx>, location: Location) -> Vec<Place<'tcx>> {
    let mut visitor = RvaluePlacesVisitor { places: Vec::new() };
    visitor.visit_rvalue(rvalue, location);
    visitor.places
}

struct RvaluePlacesVisitor<'tcx> {
    places: Vec<Place<'tcx>>,
}

impl<'tcx> Visitor<'tcx> for RvaluePlacesVisitor<'tcx> {
    fn visit_place(&mut self, place: &Place<'tcx>, _context: PlaceContext, _location: Location) {
        self.places.push(*place);
    }
}
